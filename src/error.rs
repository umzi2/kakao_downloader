use std::{ error::Error as StdError, fmt };

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug)]
pub struct Error {
    context: String,
    source: Box<ErrorKind>,
}

impl Error {
    pub fn new(context: impl Into<String>, source: impl Into<ErrorKind>) -> Self {
        Self {
            context: context.into(),
            source: Box::new(source.into()),
        }
    }

    pub fn unexpected(context: impl Into<String>) -> Self {
        Self::new(context, ErrorKind::Unexpected)
    }

    pub fn rejected(context: impl Into<String>) -> Self {
        Self::new(context, ErrorKind::Rejected)
    }

    pub fn unauthorized(context: impl Into<String>) -> Self {
        Self::new(context, ErrorKind::Unauthorized)
    }

    pub fn kind(&self) -> &ErrorKind {
        &self.source
    }

    pub fn is_auth_problem(&self) -> bool {
        matches!(self.kind(), ErrorKind::Unauthorized)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let source = self.source.to_string();

        if self.context.is_empty() {
            f.write_str(&source)
        } else if source.is_empty() {
            f.write_str(&self.context)
        } else {
            write!(f, "{}: {source}", self.context)
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.source)
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub enum ErrorKind {
    Io(std::io::Error),
    Http(reqwest::Error),
    Url(url::ParseError),
    Image(image::ImageError),
    ConfigRead(ron::error::SpannedError),
    ConfigWrite(ron::Error),
    Task(tokio::task::JoinError),
    Unauthorized,
    Unexpected,
    Rejected,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "ошибка ввода-вывода: {error}"),
            Self::Http(error) => write!(f, "ошибка HTTP: {error}"),
            Self::Url(error) => write!(f, "некорректный URL: {error}"),
            Self::Image(error) => write!(f, "ошибка изображения: {error}"),
            Self::ConfigRead(error) => write!(f, "ошибка разбора RON: {error}"),
            Self::ConfigWrite(error) => write!(f, "ошибка сериализации RON: {error}"),
            Self::Task(error) => write!(f, "фоновая задача завершилась ошибкой: {error}"),
            Self::Unauthorized => Ok(()),
            Self::Unexpected => Ok(()),
            Self::Rejected => f.write_str("операция отклонена"),
        }
    }
}

impl StdError for ErrorKind {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Http(error) => Some(error),
            Self::Url(error) => Some(error),
            Self::Image(error) => Some(error),
            Self::ConfigRead(error) => Some(error),
            Self::ConfigWrite(error) => Some(error),
            Self::Task(error) => Some(error),
            Self::Unauthorized | Self::Unexpected | Self::Rejected => None,
        }
    }
}

macro_rules! from_error_kind {
    ($($source:ty => $kind:ident),+ $(,)?) => {
        $(
            impl From<$source> for ErrorKind {
                fn from(source: $source) -> Self {
                    Self::$kind(source)
                }
            }
        )+
    };
}

from_error_kind! {
    std::io::Error => Io,
    reqwest::Error => Http,
    url::ParseError => Url,
    image::ImageError => Image,
    ron::error::SpannedError => ConfigRead,
    ron::Error => ConfigWrite,
    tokio::task::JoinError => Task,
}

pub trait Context<T> {
    fn context(self, context: impl Into<String>) -> Result<T>;
}

impl<T, E: Into<ErrorKind>> Context<T> for std::result::Result<T, E> {
    fn context(self, context: impl Into<String>) -> Result<T> {
        self.map_err(|source| Error::new(context, source))
    }
}

impl<T> Context<T> for Option<T> {
    fn context(self, context: impl Into<String>) -> Result<T> {
        self.ok_or_else(|| Error::unexpected(context))
    }
}
