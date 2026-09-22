use std::{ fs, path::{ Path, PathBuf } };

use serde::{ Deserialize, Serialize };

use crate::error::{ Context, Error, Result };

pub const CONFIG_PATH: &str = "config.ron";

pub const DEFAULT_SPREAD: usize = 11_000;

pub const DEFAULT_STEP: usize = 1;

pub const DEFAULT_OUT_DIR: &str = "tmg";

const MIN_SPREAD: usize = 500;

fn default_spread() -> usize {
    DEFAULT_SPREAD
}

fn default_step() -> usize {
    DEFAULT_STEP
}

fn default_out_dir() -> String {
    DEFAULT_OUT_DIR.to_owned()
}

fn default_parallel() -> bool {
    true
}

pub fn auto_tolerance(spread: usize) -> usize {
    spread / 8
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub cookie: String,
    #[serde(default = "default_spread")]
    pub spread: usize,
    #[serde(default = "default_step")]
    pub step: usize,
    #[serde(default = "default_out_dir")]
    pub out_dir: String,
    #[serde(default)]
    pub tolerance: Option<usize>,
    #[serde(default = "default_parallel")]
    pub parallel: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cookie: String::new(),
            spread: DEFAULT_SPREAD,
            step: DEFAULT_STEP,
            out_dir: DEFAULT_OUT_DIR.to_owned(),
            tolerance: None,
            parallel: true,
        }
    }
}

impl Config {
    pub fn exists() -> bool {
        Path::new(CONFIG_PATH).is_file()
    }

    pub fn load() -> Result<Self> {
        let data = fs::read_to_string(CONFIG_PATH).context(format!("чтение {CONFIG_PATH}"))?;
        ron::from_str(&data).context(format!("разбор {CONFIG_PATH}"))
    }

    pub fn save(&self) -> Result<()> {
        let data = ron::ser
            ::to_string_pretty(self, ron::ser::PrettyConfig::new())
            .context(format!("сериализация {CONFIG_PATH}"))?;
        fs::write(CONFIG_PATH, data).context(format!("запись {CONFIG_PATH}"))
    }

    pub async fn load_async() -> Result<Self> {
        tokio::task::spawn_blocking(Self::load).await.context("задача чтения конфига")?
    }

    pub async fn save_async(&self) -> Result<()> {
        let config = self.clone();
        tokio::task::spawn_blocking(move || config.save()).await.context("задача записи конфига")?
    }

    pub fn has_cookie(&self) -> bool {
        !self.cookie.trim().is_empty()
    }

    pub fn out_dir(&self) -> PathBuf {
        PathBuf::from(self.out_dir.trim())
    }

    pub fn tolerance(&self) -> usize {
        self.tolerance.unwrap_or_else(|| auto_tolerance(self.spread))
    }

    pub fn validate(&self) -> Result<()> {
        if self.out_dir.trim().is_empty() {
            return Err(Error::unexpected("каталог вывода не заполнен"));
        }
        if self.spread < MIN_SPREAD {
            return Err(
                Error::unexpected(format!("высота фрагмента должна быть не меньше {MIN_SPREAD}"))
            );
        }
        if self.step == 0 {
            return Err(Error::unexpected("шаг сканирования должен быть не меньше 1"));
        }
        if self.tolerance() == 0 {
            return Err(Error::unexpected("разброс должен быть не меньше 1"));
        }
        if self.tolerance() > self.spread / 2 {
            return Err(
                Error::unexpected(
                    format!(
                        "разброс не должен превышать половину высоты фрагмента ({})",
                        self.spread / 2
                    )
                )
            );
        }
        Ok(())
    }
}
