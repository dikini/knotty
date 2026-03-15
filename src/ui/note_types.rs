use crate::client::{NoteData, NoteModeAvailability, NoteType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteTypeIndicator {
    pub icon_name: String,
    pub badge: String,
}

pub fn effective_note_type(note: &NoteData) -> NoteType {
    note.note_type.unwrap_or(NoteType::Markdown)
}

pub fn available_modes_for_note_type(note_type: NoteType) -> NoteModeAvailability {
    match note_type {
        NoteType::Markdown => NoteModeAvailability {
            meta: true,
            source: true,
            edit: true,
            view: true,
        },
        NoteType::Pdf | NoteType::Image => NoteModeAvailability {
            meta: false,
            source: false,
            edit: false,
            view: true,
        },
        NoteType::Youtube => NoteModeAvailability {
            meta: true,
            source: false,
            edit: false,
            view: true,
        },
        NoteType::Unknown => NoteModeAvailability {
            meta: true,
            source: true,
            edit: false,
            view: true,
        },
    }
}

pub fn available_modes_for_note(note: &NoteData) -> NoteModeAvailability {
    note.available_modes
        .clone()
        .unwrap_or_else(|| available_modes_for_note_type(effective_note_type(note)))
}

pub fn note_type_indicator(type_badge: Option<&str>) -> NoteTypeIndicator {
    let badge_lower = type_badge.map(str::to_lowercase);
    let (icon_name, badge) = match badge_lower.as_deref() {
        Some("youtube") => ("video-x-generic", "YT"),
        Some("pdf") => ("application-pdf", "PDF"),
        Some("png") | Some("jpg") | Some("jpeg") | Some("gif") | Some("webp") | Some("svg")
        | Some("image") => ("image-x-generic", ""),
        Some("md") | Some("markdown") => ("text-x-markdown", ""),
        Some(other) => ("text-x-generic", other),
        None => ("text-x-markdown", ""),
    };

    NoteTypeIndicator {
        icon_name: icon_name.to_string(),
        badge: badge.to_uppercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::{Backlink, Heading};

    #[test]
    fn image_note_is_view_only_by_default() {
        assert_eq!(
            available_modes_for_note_type(NoteType::Image),
            NoteModeAvailability {
                meta: false,
                source: false,
                edit: false,
                view: true,
            }
        );
    }

    #[test]
    fn youtube_note_allows_meta_and_view_by_default() {
        assert_eq!(
            available_modes_for_note_type(NoteType::Youtube),
            NoteModeAvailability {
                meta: true,
                source: false,
                edit: false,
                view: true,
            }
        );
    }

    #[test]
    fn explicit_available_modes_override_type_defaults() {
        let note = test_note(
            Some(NoteType::Pdf),
            Some(NoteModeAvailability {
                meta: true,
                source: false,
                edit: false,
                view: true,
            }),
        );

        assert_eq!(
            available_modes_for_note(&note),
            NoteModeAvailability {
                meta: true,
                source: false,
                edit: false,
                view: true,
            }
        );
    }

    #[test]
    fn note_type_indicator_uses_expected_pdf_and_youtube_badges() {
        let pdf = note_type_indicator(Some("pdf"));
        let youtube = note_type_indicator(Some("youtube"));

        assert_eq!(pdf.icon_name, "application-pdf");
        assert_eq!(pdf.badge, "PDF");
        assert_eq!(youtube.icon_name, "video-x-generic");
        assert_eq!(youtube.badge, "YT");
    }

    fn test_note(
        note_type: Option<NoteType>,
        available_modes: Option<NoteModeAvailability>,
    ) -> NoteData {
        NoteData {
            id: "note-1".to_string(),
            path: "note.md".to_string(),
            title: "Note".to_string(),
            content: "# Note".to_string(),
            created_at: 0,
            modified_at: 0,
            word_count: 1,
            headings: vec![Heading {
                level: 1,
                text: "Note".to_string(),
                slug: "note".to_string(),
            }],
            backlinks: vec![Backlink {
                path: "other.md".to_string(),
                title: "Other".to_string(),
                excerpt: None,
            }],
            note_type,
            available_modes,
            metadata: None,
            embed: None,
            media: None,
            type_badge: None,
            is_dimmed: false,
        }
    }
}
