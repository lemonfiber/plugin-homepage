use lemonfiber_ports::service::{Entry, EntryWidget, Group, PageWidget, Panels};
use serde_json::{json, Value};

use super::{rendered, Refused, Rendered};

fn entry() -> Entry {
    Entry {
        name: "Jellyfin".to_owned(),
        icon: "jellyfin.png".to_owned(),
        link: "http://192.0.2.10:8096".to_owned(),
        description: "Watch your library".to_owned(),
        check: Some("http://door:8096/health".to_owned()),
        widget: Some(EntryWidget::StatusPage {
            url: "http://uptime-kuma:3001".to_owned(),
            page: "household".to_owned(),
        }),
    }
}

fn panels(entry: Entry) -> Panels {
    Panels {
        groups: vec![Group {
            name: "Watch".to_owned(),
            entries: vec![entry],
        }],
        widgets: vec![
            PageWidget::Resources,
            PageWidget::Search {
                url: "https://duckduckgo.com/?q=".to_owned(),
            },
        ],
    }
}

/// Every attempt to carry something Homepage expands, or a break in what is written.
const INJECTIONS: [&str; 10] = [
    "a\nb",
    "a\r\nb",
    "a\u{0}b",
    "{{HOMEPAGE_VAR_LAN_HOST}}",
    "{{HOMEPAGE_FILE_SECRET}}",
    "${HOME}",
    "a$b",
    "a\u{1b}[2J",
    "a\u{7f}",
    "a\tb",
];

fn read(written: &str) -> Value {
    serde_json::from_str(written).unwrap_or_default()
}

fn none() -> Rendered {
    Rendered {
        services: String::new(),
        widgets: String::new(),
    }
}

#[test]
fn every_entry_and_widget_is_written_as_homepage_reads_it() {
    let written = rendered(&panels(entry())).unwrap_or_else(|_| none());

    assert_eq!(
        read(&written.services),
        json!([{ "Watch": [{ "Jellyfin": {
            "icon": "jellyfin.png",
            "href": "http://192.0.2.10:8096",
            "description": "Watch your library",
            "siteMonitor": "http://door:8096/health",
            "widget": { "type": "uptimekuma", "url": "http://uptime-kuma:3001", "slug": "household" },
        }}]}])
    );
    assert_eq!(
        read(&written.widgets),
        json!([
            { "resources": { "cpu": true, "memory": true, "expanded": true } },
            { "search": { "provider": "custom", "url": "https://duckduckgo.com/?q=", "target": "_blank" } },
        ])
    );
}

#[test]
fn an_address_that_is_not_http_or_https_is_refused_wherever_it_is() {
    let mut addresses: Vec<String> = INJECTIONS.iter().map(|one| (*one).to_owned()).collect();
    addresses.extend(
        [
            "",
            "javascript:alert(1)",
            "JAVASCRIPT:alert(1)",
            "data:text/html,<script>",
            "vbscript:x",
            "//evil.example",
            "http://",
            "http:///etc",
            "http://a b",
            "http://a\"b",
            "http://a<b>",
            "http://a\\b",
            "http://a{b}",
            "file:///etc/passwd",
        ]
        .map(str::to_owned),
    );
    for address in addresses {
        let mut linked = entry();
        linked.link.clone_from(&address);
        assert_eq!(
            rendered(&panels(linked)),
            Err(Refused::Address(address.clone())),
            "link {address:?}"
        );
        let mut checked = entry();
        checked.check = Some(address.clone());
        assert_eq!(
            rendered(&panels(checked)),
            Err(Refused::Address(address.clone()))
        );
        let mut monitored = entry();
        monitored.widget = Some(EntryWidget::StatusPage {
            url: address.clone(),
            page: "p".to_owned(),
        });
        assert_eq!(
            rendered(&panels(monitored)),
            Err(Refused::Address(address.clone()))
        );
        let mut searched = panels(entry());
        searched.widgets = vec![PageWidget::Search {
            url: address.clone(),
        }];
        assert_eq!(rendered(&searched), Err(Refused::Address(address)));
    }
}

#[test]
fn an_icon_is_a_plain_name_or_an_address() {
    for good in [
        "jellyfin.png",
        "mdi-heart-pulse",
        "si_x",
        "https://icons.example/a.svg",
    ] {
        let mut iconned = entry();
        iconned.icon = good.to_owned();
        assert!(rendered(&panels(iconned)).is_ok(), "{good}");
    }
    for bad in ["", ".hidden", "a b", "javascript:alert(1)", "{{X}}", "a/b"] {
        let mut iconned = entry();
        iconned.icon = bad.to_owned();
        assert_eq!(
            rendered(&panels(iconned)),
            Err(Refused::Icon(bad.to_owned()))
        );
    }
}

#[test]
fn text_holding_a_control_character_or_an_expansion_is_refused() {
    for bad in INJECTIONS {
        let mut named = entry();
        named.name = bad.to_owned();
        assert_eq!(rendered(&panels(named)), Err(Refused::Text(bad.to_owned())));
        let mut described = entry();
        described.description = bad.to_owned();
        assert_eq!(
            rendered(&panels(described)),
            Err(Refused::Text(bad.to_owned()))
        );
        let mut grouped = panels(entry());
        if let Some(group) = grouped.groups.first_mut() {
            group.name = bad.to_owned();
        }
        assert_eq!(rendered(&grouped), Err(Refused::Text(bad.to_owned())));
        let mut paged = entry();
        paged.widget = Some(EntryWidget::StatusPage {
            url: "http://uptime-kuma:3001".to_owned(),
            page: bad.to_owned(),
        });
        assert_eq!(rendered(&panels(paged)), Err(Refused::Text(bad.to_owned())));
    }
}

#[test]
fn text_with_quotes_and_markup_is_written_quoted_and_reads_back_as_it_was() {
    let mut quoted = entry();
    quoted.description = "\"Films\" & <shows>: it's 'all' here #1".to_owned();
    let written = rendered(&panels(quoted.clone())).unwrap_or_else(|_| none());

    assert_eq!(
        read(&written.services).pointer("/0/Watch/0/Jellyfin/description"),
        Some(&json!(quoted.description))
    );
}

#[test]
fn a_value_longer_than_any_label_is_refused() {
    let mut long = entry();
    long.description = "a".repeat(2049);
    assert!(matches!(rendered(&panels(long)), Err(Refused::Text(_))));
}
