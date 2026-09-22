use std::sync::Arc;

use tokio::{
    sync::mpsc::{ UnboundedReceiver, UnboundedSender, unbounded_channel },
    task::JoinHandle,
};

use crate::{
    download::{ Outcome, Progress, SliceOptions, download_chapter },
    kakao::{ ClientEvent, KakaoClient, chapters::chapter::Chapter },
    retry::with_retries,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryAction {
    Chapters {
        title_id: usize,
    },
    Run,
}

#[derive(Debug)]
pub enum Message {
    ChaptersPage {
        title_id: usize,
        cursor: usize,
        chapters: Vec<Chapter>,
        next_cursor: Option<usize>,
        total: usize,
    },
    Failed {
        retry: RetryAction,
        error: String,
        auth: bool,
    },
    Progress(Progress),
    Finished(Outcome),
    Client(ClientEvent),
}

pub fn client_events() -> (UnboundedSender<ClientEvent>, UnboundedReceiver<ClientEvent>) {
    unbounded_channel()
}

pub fn spawn_client_forwarder(
    mut events: UnboundedReceiver<ClientEvent>,
    messages: UnboundedSender<Message>
) {
    tokio::spawn(async move {
        while let Some(event) = events.recv().await {
            if messages.send(Message::Client(event)).is_err() {
                break;
            }
        }
    });
}

pub fn spawn_chapters_page(
    client: Arc<KakaoClient>,
    title_id: usize,
    cursor: usize,
    messages: UnboundedSender<Message>
) {
    tokio::spawn(async move {
        let result = with_retries(|| client.chapter_page(title_id, cursor)).await;

        let message = match result {
            Ok(page) =>
                Message::ChaptersPage {
                    title_id,
                    cursor,
                    chapters: page.chapters,
                    next_cursor: page.next_cursor,
                    total: page.total,
                },
            Err(error) =>
                Message::Failed {
                    retry: RetryAction::Chapters { title_id },
                    auth: error.is_auth_problem(),
                    error: error.to_string(),
                },
        };

        let _ = messages.send(message);
    });
}

pub fn spawn_run(
    client: Arc<KakaoClient>,
    title_id: usize,
    chapter_index: usize,
    options: SliceOptions,
    parallel: bool,
    messages: UnboundedSender<Message>
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let progress_sender = messages.clone();
        let result = download_chapter(
            &client,
            title_id,
            chapter_index,
            &options,
            parallel,
            move |progress| {
                let _ = progress_sender.send(Message::Progress(progress));
            }
        ).await;

        let message = match result {
            Ok(outcome) => Message::Finished(outcome),
            Err(error) =>
                Message::Failed {
                    retry: RetryAction::Run,
                    auth: error.is_auth_problem(),
                    error: error.to_string(),
                },
        };

        let _ = messages.send(message);
    })
}
