//! Homepage, told what it shows through the two files it reads in its configuration
//! directory: its services, grouped, and the widgets along its page.
//!
//! Each file is written as JSON, which is YAML as Homepage reads it, so serde quotes every
//! value and what is written reads back exactly. Every value is held to its grammar
//! before anything is written. The panel list each write came from is kept beside the
//! files, and a file that is neither the one last written nor the one the plugin ships
//! is someone's own: it is named in the refusal and never written over.

pub mod files;

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use lemonfiber_adapter_kit::shared::{Shared, SharedName, Unshared};
use lemonfiber_ports::service::{Failure, Panels, Showing};

use self::files::{Refused, SERVICES, WIDGETS};

/// What a failure names Homepage as.
const SERVICE: &str = "Homepage";

/// The mode every file is written with: Homepage reads it, and only the adapter writes it.
const MODE: u32 = 0o644;

/// The panel list the files were last written from, beside them.
pub const WRITTEN: &str = "lemonfiber-panels.json";

/// The services file the plugin ships, which lemonfiber writes before Homepage first
/// starts.
const SHIPPED_SERVICES: &str = include_str!("../config/homepage/services.yaml");

/// The widgets file the plugin ships.
const SHIPPED_WIDGETS: &str = include_str!("../config/homepage/widgets.yaml");

/// One Homepage, told what to show through its configuration directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Homepage {
    directory: PathBuf,
}

impl Homepage {
    /// The Homepage whose configuration directory is shared at `directory`.
    #[must_use]
    pub fn at(directory: &Path) -> Self {
        Self {
            directory: directory.to_path_buf(),
        }
    }

    /// Whether Homepage's configuration directory is there to write.
    pub async fn answers(&self) -> bool {
        tokio::fs::metadata(&self.directory)
            .await
            .is_ok_and(|held| held.is_dir())
    }

    /// What `file` holds, or none where it is not there.
    async fn read(&self, file: &str) -> Result<Option<String>, Failure> {
        Shared::at(&self.directory)
            .read(&SharedName::new(file).map_err(unshared)?)
            .await
            .map_err(unshared)?
            .map(|bytes| {
                String::from_utf8(bytes).map_err(|_| refused(&format!("{file} is not text")))
            })
            .transpose()
    }

    /// Write `contents` to `file` in Homepage's configuration directory, in one step.
    async fn put(&self, file: &str, contents: String) -> Result<(), Failure> {
        Shared::at(&self.directory)
            .write(
                &SharedName::new(file).map_err(unshared)?,
                contents.into_bytes(),
                MODE,
            )
            .await
            .map_err(unshared)
    }

    /// The panel list last written, or none where nothing was.
    async fn written(&self) -> Result<Option<Panels>, Failure> {
        self.read(WRITTEN)
            .await?
            .map(|text| {
                serde_json::from_str(&text)
                    .map_err(|_| refused("the panel list written could not be read"))
            })
            .transpose()
    }

    /// The first file that is someone's own: neither the one last written from
    /// `written`, nor the one the plugin ships, nor absent.
    async fn edited(&self, written: Option<&Panels>) -> Result<Option<&'static str>, Failure> {
        let expected = written
            .map(files::rendered)
            .transpose()
            .map_err(|why| refused(&said(&why)))?;
        for (file, shipped, ours) in [
            (
                SERVICES,
                SHIPPED_SERVICES,
                expected.as_ref().map(|one| one.services.as_str()),
            ),
            (
                WIDGETS,
                SHIPPED_WIDGETS,
                expected.as_ref().map(|one| one.widgets.as_str()),
            ),
        ] {
            let Some(held) = self.read(file).await? else {
                continue;
            };
            if held != shipped && Some(held.as_str()) != ours {
                return Ok(Some(file));
            }
        }
        Ok(None)
    }
}

#[async_trait]
impl Showing for Homepage {
    async fn apply(&self, panels: &Panels) -> Result<(), Failure> {
        let rendered = files::rendered(panels).map_err(|why| refused(&said(&why)))?;
        let written = self.written().await?;
        if let Some(file) = self.edited(written.as_ref()).await? {
            return Err(refused(&format!(
                "{file} was changed by hand, so it is left as it is"
            )));
        }
        let record = serde_json::to_string(panels).unwrap_or_default();
        self.put(WRITTEN, record).await?;
        self.put(SERVICES, rendered.services).await?;
        self.put(WIDGETS, rendered.widgets).await?;
        if &self.panels().await? == panels {
            Ok(())
        } else {
            Err(refused("the panel list written reads back as another"))
        }
    }

    async fn panels(&self) -> Result<Panels, Failure> {
        let written = self.written().await?;
        if let Some(file) = self.edited(written.as_ref()).await? {
            return Err(refused(&format!(
                "{file} was changed by hand, so what it shows is not lemonfiber's"
            )));
        }
        Ok(written.unwrap_or(Panels {
            groups: Vec::new(),
            widgets: Vec::new(),
        }))
    }
}

/// What a refusal of the panel list says.
fn said(why: &Refused) -> String {
    match why {
        Refused::Address(value) => format!("{value:?} is not an http or https address"),
        Refused::Icon(value) => format!("{value:?} is not an icon's name or address"),
        Refused::Text(value) => {
            format!("{value:?} holds what Homepage would expand or a character no label carries")
        }
    }
}

/// A refusal saying `detail`.
fn refused(detail: &str) -> Failure {
    Failure::Refused {
        service: SERVICE.to_owned(),
        detail: detail.to_owned(),
    }
}

/// A file in Homepage's configuration directory that could not be reached: a link or
/// a file that is not plain is refused by name, and a directory that is not there to
/// read or write leaves Homepage unavailable.
fn unshared(why: Unshared) -> Failure {
    match why {
        Unshared::Failed(..) => Failure::Unavailable {
            service: SERVICE.to_owned(),
        },
        refusal => refused(&refusal.to_string()),
    }
}

#[cfg(test)]
mod tests;
