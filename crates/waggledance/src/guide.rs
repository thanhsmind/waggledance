//! The built-in guide (`/guide`) — bee explained in Vietnamese, from nothing.
//!
//! guide-vi: the beehive product description is complete and unreadable to
//! anyone who does not already know bee. This section is the other half: the
//! same concepts, told to someone who knows none of them, in their own
//! language, picture first.
//!
//! **Why plain HTML rather than markdown.** Every other page of prose in this
//! product is a *project's* file, rendered through
//! [`waggledance_core::render`] — which rewrites links against a project root
//! and an index, both of which a built-in page has none of. More to the
//! point, the guide's whole method is the diagram: markdown can carry a
//! paragraph but not a labelled SVG whose boxes are the chapter links. So a
//! chapter is an HTML fragment authored by hand, embedded in the binary with
//! `include_str!`, and served inside the app's own chrome. Nothing here
//! touches the markdown pipeline, and the guide reads identically on a host
//! with no project registered and no network.
//!
//! **What a chapter may contain.** A fragment is trusted authored markup, not
//! user content — it is never sanitized, so nothing but this directory may
//! ever reach [`Chapter::body`]. It uses the app's design tokens (`--color-*`,
//! `--space-*`) and `currentColor` in SVG, so both themes come free.

/// One chapter of the guide.
pub struct Chapter {
    /// URL segment: `/guide/<slug>`. Stable — chapters are linked to each
    /// other by slug from inside their own bodies.
    pub slug: &'static str,
    /// Display number. The reading order is [`CHAPTERS`]' own order; this is
    /// what the reader sees beside the title.
    pub number: usize,
    /// Vietnamese chapter title, without the number.
    pub title: &'static str,
    /// One line naming the question the chapter answers — shown on the index
    /// card and under the chapter's own heading.
    pub blurb: &'static str,
    /// The chapter's HTML fragment (trusted, unsanitized — see module docs).
    pub body: &'static str,
}

/// Every chapter, in reading order. The order here IS the guide's order:
/// [`neighbours`] reads prev/next straight off it, and the index page lists
/// it as-is.
pub const CHAPTERS: &[Chapter] = &[
    Chapter {
        slug: "bee-la-gi",
        number: 1,
        title: "Bee là gì",
        blurb: "Ai mới thật sự là người dùng bee — và vì sao điều đó đổi mọi thứ.",
        body: include_str!("../assets/guide/vi/bee-la-gi.html"),
    },
    Chapter {
        slug: "tu-vung",
        number: 2,
        title: "Từ vựng",
        blurb: "Khoảng 60 chữ bạn sẽ gặp đi gặp lại, mỗi chữ một dòng.",
        body: include_str!("../assets/guide/vi/tu-vung.html"),
    },
    Chapter {
        slug: "cong-gate",
        number: 3,
        title: "Năm cái cổng",
        blurb: "Những chỗ duy nhất bee dừng lại chờ bạn nói \"được\".",
        body: include_str!("../assets/guide/vi/cong-gate.html"),
    },
    Chapter {
        slug: "kho-store",
        number: 4,
        title: "Kho .bee/",
        blurb: "Bee ghi cái gì xuống đĩa, và vì sao bạn không được sửa tay.",
        body: include_str!("../assets/guide/vi/kho-store.html"),
    },
    Chapter {
        slug: "phien-session",
        number: 5,
        title: "Phiên làm việc",
        blurb: "Preamble, heartbeat, handoff — bộ khung quanh một phiên agent.",
        body: include_str!("../assets/guide/vi/phien-session.html"),
    },
    Chapter {
        slug: "hooks-guards",
        number: 6,
        title: "Hooks — người gác cổng",
        blurb: "Ai thật sự chặn được một lần ghi file, và chặn bằng cách nào.",
        body: include_str!("../assets/guide/vi/hooks-guards.html"),
    },
    Chapter {
        slug: "worktree",
        number: 7,
        title: "Worktree & staging",
        blurb: "Vì sao mỗi việc nên có một bàn làm việc riêng.",
        body: include_str!("../assets/guide/vi/worktree.html"),
    },
    Chapter {
        slug: "vong-doi",
        number: 8,
        title: "Vòng đời một feature",
        blurb: "Sáu chặng từ ý tưởng tới đóng sổ, và chặng nào cần gì.",
        body: include_str!("../assets/guide/vi/vong-doi.html"),
    },
    Chapter {
        slug: "wayfinding",
        number: 9,
        title: "Dò đường (wayfinding)",
        blurb: "Cửa thứ hai, cho một ý tưởng còn mù mờ chưa đặt được đích.",
        body: include_str!("../assets/guide/vi/wayfinding.html"),
    },
    Chapter {
        slug: "cell-lane",
        number: 10,
        title: "Cell & Lane",
        blurb: "Đơn vị việc nhỏ nhất, và cỡ việc quyết định bao nhiêu nghi thức.",
        body: include_str!("../assets/guide/vi/cell-lane.html"),
    },
    Chapter {
        slug: "giao-viec",
        number: 11,
        title: "Giao việc cho subagent",
        blurb: "Dispatch, worker, herding — một cửa duy nhất để gọi quân.",
        body: include_str!("../assets/guide/vi/giao-viec.html"),
    },
    Chapter {
        slug: "phoi-hop",
        number: 12,
        title: "Nhiều phiên cùng lúc",
        blurb: "Claim, reservation, hold — cách nhiều agent không giẫm chân nhau.",
        body: include_str!("../assets/guide/vi/phoi-hop.html"),
    },
    Chapter {
        slug: "bo-nho",
        number: 13,
        title: "Bee nhớ những gì",
        blurb: "Capture, decision, knowledge, backlog — bốn tầng trí nhớ.",
        body: include_str!("../assets/guide/vi/bo-nho.html"),
    },
    Chapter {
        slug: "config",
        number: 14,
        title: "Cấu hình chi tiết",
        blurb: "Mọi key trong .bee/config.json: giá trị mặc định, ý nghĩa, ví dụ.",
        body: include_str!("../assets/guide/vi/config.html"),
    },
    Chapter {
        slug: "dung-hieu-qua",
        number: 15,
        title: "Dùng bee cho hiệu quả",
        blurb: "Công thức thực chiến, và những chỗ người mới hay vấp.",
        body: include_str!("../assets/guide/vi/dung-hieu-qua.html"),
    },
];

/// The index page's own opening — the big picture, above the chapter cards.
pub const OVERVIEW: &str = include_str!("../assets/guide/vi/_index.html");

/// The chapter a `/guide/<slug>` address names, or `None` — the handler
/// answers an unknown slug with the guide's own 404, never a redirect, so a
/// wrong link is visible rather than silently swallowed by chapter one.
pub fn find(slug: &str) -> Option<&'static Chapter> {
    CHAPTERS.iter().find(|c| c.slug == slug)
}

/// The chapters either side of `slug` in reading order, for the footer's
/// prev/next pair. Both ends are `None` at their own end of the list.
pub fn neighbours(slug: &str) -> (Option<&'static Chapter>, Option<&'static Chapter>) {
    let Some(at) = CHAPTERS.iter().position(|c| c.slug == slug) else {
        return (None, None);
    };
    let prev = if at == 0 {
        None
    } else {
        CHAPTERS.get(at - 1)
    };
    (prev, CHAPTERS.get(at + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The slug is the address AND the link target every other chapter writes
    /// by hand. A duplicate would make one of them unreachable, and the
    /// numbering is what the reader navigates by — both are cheap to assert
    /// and expensive to notice by eye.
    #[test]
    fn every_chapter_has_a_unique_slug_and_its_number_matches_its_position() {
        let mut seen = std::collections::HashSet::new();
        for (i, c) in CHAPTERS.iter().enumerate() {
            assert!(
                seen.insert(c.slug),
                "two chapters claim the slug {}",
                c.slug
            );
            assert_eq!(
                c.number,
                i + 1,
                "chapter {} is numbered {} but sits at position {}",
                c.slug,
                c.number,
                i + 1
            );
            assert!(
                !c.title.trim().is_empty() && !c.blurb.trim().is_empty(),
                "chapter {} must carry a title and a blurb",
                c.slug
            );
        }
    }

    /// A chapter whose body never made it out of the template is worse than a
    /// missing chapter: the menu promises it and the page serves whitespace.
    /// The floor is deliberately low — this catches an empty or stub file,
    /// not thin writing.
    #[test]
    fn every_chapter_body_carries_real_content() {
        for c in CHAPTERS {
            assert!(
                c.body.len() > 400,
                "chapter {} looks like a stub ({} bytes)",
                c.slug,
                c.body.len()
            );
            assert!(
                c.body.contains('<'),
                "chapter {} must be an HTML fragment",
                c.slug
            );
        }
        assert!(OVERVIEW.len() > 400, "the guide overview looks like a stub");
    }

    /// Every `/guide/<slug>` a chapter body links to must name a chapter that
    /// exists. The cross-links ARE the feature — a dead one is the failure
    /// this guide was written to avoid, and no compiler catches a string.
    #[test]
    fn every_cross_link_between_chapters_resolves() {
        let mut bodies: Vec<(&str, &str)> = CHAPTERS.iter().map(|c| (c.slug, c.body)).collect();
        bodies.push(("_index", OVERVIEW));
        for (from, body) in bodies {
            for (at, _) in body.match_indices("/guide/") {
                let rest = &body[at + "/guide/".len()..];
                let end = rest
                    .find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '-'))
                    .unwrap_or(rest.len());
                let slug = &rest[..end];
                if slug.is_empty() {
                    continue; // a link to the guide's own index
                }
                assert!(
                    find(slug).is_some(),
                    "{from} links to /guide/{slug}, which is not a chapter"
                );
            }
        }
    }

    /// D2: a chapter opens on a picture. That is the guide's whole method, and
    /// a method that lives only in a CONTEXT.md sentence is one the next
    /// chapter quietly drops — so it is a test. Each figure must also carry
    /// its own `<title>`, since a diagram doing the explaining is unreadable
    /// to a screen reader without one.
    #[test]
    fn every_chapter_opens_on_a_figure_and_every_figure_names_itself() {
        for c in CHAPTERS.iter().chain(std::iter::once(&Chapter {
            slug: "_index",
            number: 0,
            title: "Bức tranh lớn",
            blurb: "",
            body: OVERVIEW,
        })) {
            let figures = c.body.matches(r#"<figure class="guide-fig">"#).count();
            assert!(
                figures > 0,
                "chapter {} explains without a single picture",
                c.slug
            );
            assert_eq!(
                c.body.matches("<svg").count(),
                c.body.matches("</svg>").count(),
                "chapter {} has an unclosed svg",
                c.slug
            );
            assert!(
                c.body.matches("<svg").count() >= figures,
                "chapter {} has a guide-fig with no drawing in it",
                c.slug
            );
            assert_eq!(
                c.body.matches("<svg").count(),
                c.body.matches("<title id=").count(),
                "every svg in chapter {} must carry its own <title id=…> for a screen reader",
                c.slug
            );
            // A literal colour or an inline style in a diagram is a diagram
            // that goes blind on the other side of the theme toggle. The
            // `fig-*` classes are the whole palette (see app.css).
            for banned in ["fill=\"#", "stroke=\"#", "style=\"", "fill=\"rgb", "stroke=\"rgb"] {
                assert!(
                    !c.body.contains(banned),
                    "chapter {} hard-codes {banned} — use the fig-* classes so both themes work",
                    c.slug
                );
            }
        }
    }

    /// A fragment is embedded verbatim into a page that already has a `<main>`
    /// and a `<h1>` of its own, so one that ships its own document scaffolding
    /// would nest a second one inside the first.
    #[test]
    fn no_chapter_ships_its_own_page_scaffolding() {
        for c in CHAPTERS {
            for tag in ["<html", "<head", "<body", "<!doctype", "<main"] {
                assert!(
                    !c.body.to_ascii_lowercase().contains(tag),
                    "chapter {} must be a fragment, but carries {tag}",
                    c.slug
                );
            }
        }
    }

    #[test]
    fn neighbours_walk_the_reading_order_and_stop_at_both_ends() {
        let first = CHAPTERS.first().unwrap();
        let last = CHAPTERS.last().unwrap();
        let (prev, next) = neighbours(first.slug);
        assert!(prev.is_none(), "the first chapter has nothing before it");
        assert_eq!(next.map(|c| c.slug), Some(CHAPTERS[1].slug));

        let (prev, next) = neighbours(last.slug);
        assert!(next.is_none(), "the last chapter has nothing after it");
        assert_eq!(prev.map(|c| c.slug), Some(CHAPTERS[CHAPTERS.len() - 2].slug));

        let (prev, next) = neighbours("khong-co-chuong-nay");
        assert!(
            prev.is_none() && next.is_none(),
            "an unknown slug has no neighbours"
        );
    }
}
