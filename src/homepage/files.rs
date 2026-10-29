//! A panel list as the two files Homepage reads, every value held to its grammar first.
//!
//! Every value comes from a `dashboard.show@1` request, so from every plugin's entries.
//! An address is `http` or `https` and nothing a browser runs; an icon is a name of
//! letters, digits, `.`, `_` and `-`, or such an address; every other value is text with
//! no control character and nothing Homepage expands from its environment. Each file is
//! JSON that serde quotes, so no value can carry structure of its own.

use std::collections::BTreeMap;

use lemonfiber_ports::service::{Entry, EntryWidget, PageWidget, Panels};
use serde::Serialize;

/// Homepage's services, in its configuration directory.
pub const SERVICES: &str = "services.yaml";

/// The widgets along its page, in its configuration directory.
pub const WIDGETS: &str = "widgets.yaml";

/// Where a search from its search box opens.
const SEARCH_TARGET: &str = "_blank";

/// The search provider Homepage sends to an address it is given.
const SEARCH_PROVIDER: &str = "custom";

/// The longest value written.
const LONGEST: usize = 2048;

/// Why a panel list was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refused {
    /// An address that is not `http` or `https`, or carries what an address does not.
    Address(String),
    /// An icon that is neither a plain name nor an address.
    Icon(String),
    /// Text holding a control character or something Homepage expands.
    Text(String),
}

/// The two files, as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    /// The services file.
    pub services: String,
    /// The widgets file.
    pub widgets: String,
}

/// The files Homepage reads for `panels`.
///
/// # Errors
///
/// [`Refused`] naming the first value that is not of its grammar.
pub fn rendered(panels: &Panels) -> Result<Rendered, Refused> {
    let services = panels
        .groups
        .iter()
        .map(|group| {
            let entries = group
                .entries
                .iter()
                .map(|entry| Ok(BTreeMap::from([(text(&entry.name)?, service(entry)?)])))
                .collect::<Result<Vec<_>, Refused>>()?;
            Ok(BTreeMap::from([(text(&group.name)?, entries)]))
        })
        .collect::<Result<Vec<_>, Refused>>()?;
    let widgets = panels
        .widgets
        .iter()
        .map(widget)
        .collect::<Result<Vec<_>, Refused>>()?;
    Ok(Rendered {
        services: written(&services),
        widgets: written(&widgets),
    })
}

/// One entry as Homepage reads it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Service {
    icon: String,
    href: String,
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    site_monitor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    widget: Option<ServiceWidget>,
}

/// A widget on one entry, as Homepage names it.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
enum ServiceWidget {
    /// A monitor's status page.
    Uptimekuma { url: String, slug: String },
}

/// A widget along the page, as Homepage names it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Widget {
    /// The machine's processor and memory use.
    Resources {
        cpu: bool,
        memory: bool,
        expanded: bool,
    },
    /// A web search box.
    Search {
        provider: String,
        url: String,
        target: String,
    },
}

/// One entry as Homepage reads it, every value held to its grammar.
fn service(entry: &Entry) -> Result<Service, Refused> {
    Ok(Service {
        icon: icon(&entry.icon)?,
        href: address(&entry.link)?,
        description: text(&entry.description)?,
        site_monitor: entry.check.as_deref().map(address).transpose()?,
        widget: entry
            .widget
            .as_ref()
            .map(|widget| match widget {
                EntryWidget::StatusPage { url, page } => Ok(ServiceWidget::Uptimekuma {
                    url: address(url)?,
                    slug: text(page)?,
                }),
            })
            .transpose()?,
    })
}

/// One page widget as Homepage reads it.
fn widget(widget: &PageWidget) -> Result<Widget, Refused> {
    Ok(match widget {
        PageWidget::Resources => Widget::Resources {
            cpu: true,
            memory: true,
            expanded: true,
        },
        PageWidget::Search { url } => Widget::Search {
            provider: SEARCH_PROVIDER.to_owned(),
            url: address(url)?,
            target: SEARCH_TARGET.to_owned(),
        },
    })
}

/// `value`, where it is an `http` or `https` address of the characters an address is
/// written in, with nothing Homepage expands.
fn address(value: &str) -> Result<String, Refused> {
    let lower = value.to_ascii_lowercase();
    let rest = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"));
    let plain = value.len() <= LONGEST
        && rest.is_some_and(|rest| !rest.is_empty() && !rest.starts_with('/'))
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'-' | b'.'
                        | b'_'
                        | b'~'
                        | b':'
                        | b'/'
                        | b'?'
                        | b'#'
                        | b'['
                        | b']'
                        | b'@'
                        | b'!'
                        | b'&'
                        | b'\''
                        | b'('
                        | b')'
                        | b'*'
                        | b'+'
                        | b','
                        | b';'
                        | b'='
                        | b'%'
                )
        })
        && !value.contains("{{");
    if plain {
        Ok(value.to_owned())
    } else {
        Err(Refused::Address(value.to_owned()))
    }
}

/// `value`, where it is an icon's plain name or address.
fn icon(value: &str) -> Result<String, Refused> {
    let named = !value.is_empty()
        && value.len() <= LONGEST
        && !value.starts_with('.')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'));
    if named {
        Ok(value.to_owned())
    } else {
        address(value).map_err(|_| Refused::Icon(value.to_owned()))
    }
}

/// `value`, where it holds no control character and nothing Homepage expands.
fn text(value: &str) -> Result<String, Refused> {
    let plain = value.len() <= LONGEST
        && !value.chars().any(char::is_control)
        && !value.contains("{{")
        && !value.contains('$');
    if plain {
        Ok(value.to_owned())
    } else {
        Err(Refused::Text(value.to_owned()))
    }
}

/// `value` as the file holds it.
fn written(value: &impl Serialize) -> String {
    serde_json::to_string_pretty(value).unwrap_or_default() + "\n"
}

#[cfg(test)]
mod tests;
