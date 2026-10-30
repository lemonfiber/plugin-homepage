use std::path::{Path, PathBuf};

use lemonfiber_ports::service::{Entry, Failure, Group, PageWidget, Panels, Showing};

use super::files::{SERVICES, WIDGETS};
use super::{Homepage, SHIPPED_SERVICES, SHIPPED_WIDGETS, WRITTEN};

/// A configuration directory of its own, removed when dropped.
struct Directory(PathBuf);

impl Directory {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("homepage-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        let _ = std::fs::create_dir_all(&path);
        Self(path)
    }

    fn shipped(name: &str) -> Self {
        let directory = Self::new(name);
        let _ = std::fs::write(directory.0.join(SERVICES), SHIPPED_SERVICES);
        let _ = std::fs::write(directory.0.join(WIDGETS), SHIPPED_WIDGETS);
        directory
    }

    fn file(&self, name: &str) -> String {
        std::fs::read_to_string(self.0.join(name)).unwrap_or_default()
    }
}

impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn panels() -> Panels {
    Panels {
        groups: vec![Group {
            name: "Watch".to_owned(),
            entries: vec![Entry {
                name: "Jellyfin".to_owned(),
                icon: "jellyfin.png".to_owned(),
                link: "http://192.0.2.10:8096".to_owned(),
                description: "Watch your library".to_owned(),
                check: None,
                widget: None,
            }],
        }],
        widgets: vec![PageWidget::Resources],
    }
}

fn homepage(directory: &Path) -> Homepage {
    Homepage::at(directory)
}

#[tokio::test]
async fn the_shipped_files_are_written_over_and_the_panels_read_back() {
    let directory = Directory::shipped("applied");
    let homepage = homepage(&directory.0);

    assert!(homepage.apply(&panels()).await.is_ok());

    assert_eq!(homepage.panels().await.ok(), Some(panels()));
    assert_ne!(directory.file(SERVICES), SHIPPED_SERVICES);
    assert!(!directory.file(WRITTEN).is_empty());
}

#[tokio::test]
async fn the_shipped_files_show_nothing_of_lemonfibers() {
    let directory = Directory::shipped("shipped");

    assert_eq!(
        homepage(&directory.0).panels().await.ok(),
        Some(Panels {
            groups: Vec::new(),
            widgets: Vec::new(),
        })
    );
}

#[tokio::test]
async fn a_file_changed_by_hand_is_left_as_it_is_and_named() {
    let directory = Directory::shipped("edited");
    let homepage = homepage(&directory.0);
    assert!(homepage.apply(&panels()).await.is_ok());
    let edited = "- Mine:\n    - Router:\n        href: http://192.0.2.1\n";
    let _ = std::fs::write(directory.0.join(SERVICES), edited);

    let refused = homepage.apply(&panels()).await;

    assert!(
        matches!(&refused, Err(Failure::Refused { detail, .. }) if detail.contains(SERVICES)),
        "{refused:?}"
    );
    assert_eq!(directory.file(SERVICES), edited);
    assert!(matches!(
        homepage.panels().await,
        Err(Failure::Refused { .. })
    ));
}

#[tokio::test]
async fn a_file_that_was_never_lemonfibers_is_never_written_over() {
    let directory = Directory::new("someones");
    let own = "- Mine: []\n";
    let _ = std::fs::write(directory.0.join(WIDGETS), own);

    assert!(matches!(
        homepage(&directory.0).apply(&panels()).await,
        Err(Failure::Refused { .. })
    ));
    assert_eq!(directory.file(WIDGETS), own);
    assert!(directory.file(SERVICES).is_empty());
    assert!(directory.file(WRITTEN).is_empty());
}

#[tokio::test]
async fn a_panel_list_carrying_an_expansion_is_refused_before_anything_is_written() {
    let directory = Directory::shipped("expanded");
    let mut expanded = panels();
    if let Some(entry) = expanded
        .groups
        .first_mut()
        .and_then(|group| group.entries.first_mut())
    {
        entry.link = "http://{{HOMEPAGE_VAR_LAN_HOST}}:8096".to_owned();
    }

    assert!(matches!(
        homepage(&directory.0).apply(&expanded).await,
        Err(Failure::Refused { .. })
    ));
    assert_eq!(directory.file(SERVICES), SHIPPED_SERVICES);
    assert!(directory.file(WRITTEN).is_empty());
}

#[tokio::test]
async fn a_directory_that_is_not_there_is_unavailable() {
    let homepage = homepage(Path::new("/nonexistent/homepage-configuration"));

    assert!(!homepage.answers().await);
    assert!(matches!(
        homepage.apply(&panels()).await,
        Err(Failure::Unavailable { .. })
    ));
}

#[tokio::test]
async fn a_record_that_cannot_be_read_is_refused() {
    let directory = Directory::shipped("unreadable");
    let _ = std::fs::write(directory.0.join(WRITTEN), "not json");

    assert!(matches!(
        homepage(&directory.0).panels().await,
        Err(Failure::Refused { .. })
    ));
}

#[tokio::test]
async fn a_bad_icon_or_label_is_refused_naming_it() {
    let directory = Directory::shipped("named");
    let homepage = homepage(&directory.0);
    let mut iconned = panels();
    let mut labelled = panels();
    if let Some(entry) = iconned
        .groups
        .first_mut()
        .and_then(|group| group.entries.first_mut())
    {
        entry.icon = "javascript:alert(1)".to_owned();
    }
    if let Some(group) = labelled.groups.first_mut() {
        group.name = "{{HOMEPAGE_VAR_SECRET}}".to_owned();
    }

    for (asked, named) in [
        (iconned, "javascript:alert(1)"),
        (labelled, "HOMEPAGE_VAR_SECRET"),
    ] {
        let refused = homepage.apply(&asked).await;
        assert!(
            matches!(&refused, Err(Failure::Refused { detail, .. }) if detail.contains(named)),
            "{refused:?}"
        );
    }
    assert_eq!(directory.file(SERVICES), SHIPPED_SERVICES);
}

#[tokio::test]
async fn a_file_that_is_not_plain_is_refused_naming_it() {
    let directory = Directory::new("unplain");
    let _ = std::fs::create_dir_all(directory.0.join(SERVICES));

    let refused = homepage(&directory.0).apply(&panels()).await;

    assert!(
        matches!(&refused, Err(Failure::Refused { detail, .. }) if detail.contains(SERVICES)),
        "{refused:?}"
    );
}

#[tokio::test]
async fn a_link_planted_in_the_directory_is_refused_and_never_written_through() {
    let directory = Directory::shipped("linked");
    let outside = Directory::new("linked-outside");
    let _ = std::fs::write(outside.0.join(SERVICES), SHIPPED_SERVICES);
    let _ = std::fs::remove_file(directory.0.join(SERVICES));
    let _ = std::os::unix::fs::symlink(outside.0.join(SERVICES), directory.0.join(SERVICES));
    let homepage = homepage(&directory.0);

    let refused = homepage.apply(&panels()).await;

    assert!(
        matches!(&refused, Err(Failure::Refused { detail, .. }) if detail.contains(SERVICES)),
        "{refused:?}"
    );
    assert!(matches!(
        homepage.panels().await,
        Err(Failure::Refused { .. })
    ));
    assert_eq!(outside.file(SERVICES), SHIPPED_SERVICES);
    assert!(directory.file(WRITTEN).is_empty());
}

#[tokio::test]
async fn a_file_that_is_not_text_is_refused_naming_it() {
    let directory = Directory::shipped("binary");
    let _ = std::fs::write(directory.0.join(WIDGETS), [0xff, 0xfe]);

    let refused = homepage(&directory.0).apply(&panels()).await;

    assert!(
        matches!(&refused, Err(Failure::Refused { detail, .. }) if detail.contains(WIDGETS)),
        "{refused:?}"
    );
}

#[tokio::test]
async fn a_record_holding_what_no_file_could_carry_is_refused() {
    let directory = Directory::shipped("unrenderable");
    let mut held = panels();
    if let Some(group) = held.groups.first_mut() {
        group.name = "{{HOMEPAGE_VAR_SECRET}}".to_owned();
    }
    let _ = std::fs::write(
        directory.0.join(WRITTEN),
        serde_json::to_string(&held).unwrap_or_default(),
    );

    assert!(matches!(
        homepage(&directory.0).panels().await,
        Err(Failure::Refused { .. })
    ));
}
