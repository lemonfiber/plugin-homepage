//! A panel list as the two files Homepage reads, every value held to its grammar first.
//!
//! Every value comes from a `dashboard.show@1` request, so from every plugin's entries.
//! An address is `http` or `https` and nothing a browser runs; an address Homepage's own
//! server asks, a check or a status page, is also a service on the stack's network: its
//! host is one of the panel list's `hosts`, the names the core gives a check at, each a
//! single label and never a name a container's hosts file holds, a container's id or a
//! metadata or cluster name, and it is written as it parses; a status page is such a service's
//! origin, and its name lowercase letters, digits and `-`; an icon is a name of letters,
//! digits, `.`, `_` and `-`, or such an address; every other value is text with no
//! control character and nothing Homepage expands from its environment. Each file is
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

/// The single-label names that are not a service on the stack's network: those a
/// container's own hosts file gives the loopback and IPv6's fixed addresses, and the
/// metadata and cluster names a host's search domains complete.
const NOT_SERVICES: [&str; 10] = [
    "localhost",
    "ip6-localhost",
    "ip6-loopback",
    "ip6-localnet",
    "ip6-mcastprefix",
    "ip6-allnodes",
    "ip6-allrouters",
    "metadata",
    "instance-data",
    "kubernetes",
];

/// The lengths of a container's id, short and full, each a name its hosts file or the
/// engine's resolver answers for that container.
const CONTAINER_IDS: [usize; 2] = [12, 64];

/// The schemes Homepage's server asks a service over.
const ASKED_OVER: [&str; 2] = ["http", "https"];

/// Why a panel list was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refused {
    /// An address that is not `http` or `https`, or carries what an address does not.
    Address(String),
    /// An icon that is neither a plain name nor an address.
    Icon(String),
    /// Text holding a control character or something Homepage expands.
    Text(String),
    /// An address Homepage's server would ask whose host is not one of the panel list's
    /// `hosts`, a service on the stack's network.
    Unreached(String),
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
                .map(|entry| {
                    Ok(BTreeMap::from([(
                        text(&entry.name)?,
                        service(entry, &panels.hosts)?,
                    )]))
                })
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

/// One entry as Homepage reads it, every value held to its grammar and every address its
/// server asks to `hosts`.
fn service(entry: &Entry, hosts: &[String]) -> Result<Service, Refused> {
    Ok(Service {
        icon: icon(&entry.icon)?,
        href: address(&entry.link)?,
        description: text(&entry.description)?,
        site_monitor: entry
            .check
            .as_deref()
            .map(|check| reached(check, hosts).map(String::from))
            .transpose()?,
        widget: entry
            .widget
            .as_ref()
            .map(|widget| match widget {
                EntryWidget::StatusPage { url, page } => Ok(ServiceWidget::Uptimekuma {
                    url: origin(url, hosts)?,
                    slug: slug(page)?,
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

/// `value` parsed, where it is an address of a service on the stack's network: `http` or
/// `https`, with no account and no fragment, and a host that is one of `hosts` and one
/// label of a service's name.
fn reached(value: &str, hosts: &[String]) -> Result<url::Url, Refused> {
    let asked = address(value)?;
    let parsed = url::Url::parse(&asked).map_err(|_| Refused::Unreached(asked.clone()))?;
    let served = ASKED_OVER.contains(&parsed.scheme())
        && parsed.username().is_empty()
        && parsed.password().is_none()
        && matches!(parsed.host(), Some(url::Host::Domain(name))
            if stack_name(name) && hosts.iter().any(|host| host == name));
    if !served {
        Err(Refused::Unreached(asked))
    } else if parsed.fragment().is_some() {
        Err(Refused::Address(asked))
    } else {
        Ok(parsed)
    }
}

/// The origin `value` names, `scheme://host[:port]`, where it is an address of a service on
/// the stack's network with no path and no query.
fn origin(value: &str, hosts: &[String]) -> Result<String, Refused> {
    let parsed = reached(value, hosts)?;
    if parsed.path() == "/" && parsed.query().is_none() {
        Ok(parsed.origin().ascii_serialization())
    } else {
        Err(Refused::Address(value.to_owned()))
    }
}

/// Whether `name` is one label of a service's name: lowercase letters, digits and `-`,
/// starting and ending with a letter or digit, none of the names that are not a service,
/// and not shaped as a container's id.
fn stack_name(name: &str) -> bool {
    !NOT_SERVICES.contains(&name)
        && !container_id(name)
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !name.starts_with('-')
        && !name.ends_with('-')
        && name.bytes().any(|byte| byte.is_ascii_lowercase())
}

/// Whether `name` has the shape of a container's id: lowercase hexadecimal of either
/// length the engine writes one in.
fn container_id(name: &str) -> bool {
    CONTAINER_IDS.contains(&name.len())
        && name
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// `value`, where it is a status page's name as Uptime Kuma writes one: lowercase
/// letters, digits and `-`, so the address Homepage reads from it names no other path.
fn slug(value: &str) -> Result<String, Refused> {
    let named = !value.is_empty()
        && value.len() <= LONGEST
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    if named {
        Ok(value.to_owned())
    } else {
        Err(Refused::Text(value.to_owned()))
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
