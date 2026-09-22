use std::{ path::PathBuf, sync::{ Arc, atomic::{ AtomicUsize, Ordering } } };

use tokio::{ fs::{ create_dir_all, remove_dir_all }, sync::Semaphore, task::JoinSet };

use crate::{
    error::{ Context, Error, Result },
    kakao::{ KakaoClient, chapters::chapter::Purchase },
    retry::with_retries,
    slicer::{ ImageSlicer, SlicerProgress },
};

pub const PARALLEL_PAGES: usize = 4;

const PAGES_DIR: &str = "pages";

const SLICES_DIR: &str = "slices";

#[derive(Debug, Clone)]
pub struct SliceOptions {
    pub spread: usize,
    pub tolerance: usize,
    pub step: usize,
    pub out_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub enum Progress {
    Chapter,
    Renting,
    Pages,
    Page {
        done: usize,
        total: usize,
    },
    EdgeMap {
        done: usize,
        total: usize,
    },
    Slices {
        done: usize,
        total: usize,
    },
    Cleanup,
}

#[derive(Debug, Clone)]
pub struct Outcome {
    pub pages: usize,
    pub slices: usize,
    pub pages_dir: PathBuf,
    pub slices_dir: PathBuf,
    pub pages_removed: bool,
}

pub async fn download_chapter<F>(
    client: &Arc<KakaoClient>,
    title_id: usize,
    chapter_index: usize,
    options: &SliceOptions,
    parallel: bool,
    progress: F
) -> Result<Outcome>
    where F: Fn(Progress) + Send + Sync + 'static
{
    progress(Progress::Chapter);
    let chapter = with_retries(|| client.chapter(title_id, chapter_index)).await?;

    match chapter.purchase {
        Purchase::Rent => {}
        Purchase::Unknown => {
            return Err(
                Error::unauthorized(
                    format!(
                        "глава «{}»: нет данных о покупке, похоже куки просрочились",
                        chapter.title
                    )
                )
            );
        }
        Purchase::NotPurchased => {
            if !chapter.rentable {
                return Err(
                    Error::unexpected(
                        format!("главу «{}» нельзя арендовать за билет", chapter.title)
                    )
                );
            }

            progress(Progress::Renting);
            if !with_retries(|| client.rent_chapter(chapter.id)).await? {
                return Err(
                    Error::rejected(format!("главу «{}» арендовать не удалось", chapter.title))
                );
            }
        }
    }

    progress(Progress::Pages);
    let pages = with_retries(|| client.page_list(title_id, chapter.id)).await?;
    let total = pages.len();

    let chapter_dir = options.out_dir.join(sanitize(&chapter.title, chapter.id));
    let pages_dir = chapter_dir.join(PAGES_DIR);
    let slices_dir = chapter_dir.join(SLICES_DIR);
    create_dir_all(&pages_dir).await.context(format!("создание {}", pages_dir.display()))?;

    let in_flight = if parallel { PARALLEL_PAGES } else { 1 };
    let pool = Arc::new(Semaphore::new(in_flight));
    let done = Arc::new(AtomicUsize::new(0));
    let progress = Arc::new(progress);
    let mut tasks = JoinSet::new();

    for page in pages {
        let client = Arc::clone(client);
        let save_path = pages_dir.join(format!("{:05}.jpeg", page.number));
        let pool = Arc::clone(&pool);
        let done = Arc::clone(&done);
        let progress = Arc::clone(&progress);

        tasks.spawn(async move {
            let permit = pool
                .acquire_owned().await
                .map_err(|_| Error::unexpected("очередь загрузки страниц закрыта"))?;

            let height = with_retries(|| client.download_page(&page, &save_path)).await?;
            drop(permit);

            let done = done.fetch_add(1, Ordering::Relaxed) + 1;
            progress(Progress::Page { done, total });

            Ok::<_, Error>((save_path, height))
        });
    }

    let mut height = 0u32;
    let mut page_paths = Vec::with_capacity(total);

    while let Some(joined) = tasks.join_next().await {
        let (save_path, page_height) = joined.context("задача загрузки страницы")??;
        height += page_height;
        page_paths.push(save_path);
    }

    page_paths.sort();

    let mut slicer_progress = |event: SlicerProgress| progress(Progress::from(event));
    let slices = ImageSlicer::slice_chapter(
        &page_paths,
        &slices_dir,
        options.spread,
        options.tolerance,
        options.step,
        height as usize,
        &mut slicer_progress
    ).await?;

    progress(Progress::Cleanup);
    let pages_removed = remove_dir_all(&pages_dir).await.is_ok();

    Ok(Outcome {
        pages: total,
        slices: slices.len(),
        pages_dir,
        slices_dir,
        pages_removed,
    })
}

impl From<SlicerProgress> for Progress {
    fn from(event: SlicerProgress) -> Self {
        match event {
            SlicerProgress::EdgeMap { done, total } => Self::EdgeMap { done, total },
            SlicerProgress::Slices { done, total } => Self::Slices { done, total },
        }
    }
}

pub fn sanitize(title: &str, chapter_id: usize) -> String {
    let cleaned: String = title
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | ':' | '\0') { '_' } else { c }
        })
        .collect();
    let cleaned = cleaned.trim().trim_matches('.').to_owned();

    if cleaned.is_empty() {
        format!("chapter_{chapter_id}")
    } else {
        cleaned
    }
}
