//! Visual regression harness for `docs/design/VERGE_DESIGN_SYSTEM.md`'s
//! named state compositions — Idle, Working, Permission, Completed,
//! Multiple Active, Overflow, and a Fraction-metric ring.
//!
//! This is the *only* place in this repository synthetic tool/session/tint
//! data is allowed to exist (per this task's own "real data only" rule):
//! `apps/desktop` never imports this file, and nothing here is reachable
//! from the real running application. Its sole purpose is letting a human
//! compare the actual rendered surface against the design spec without
//! forcing real accounts into artificial states.
//!
//! Run with `cargo run -p verge-platform-windows --example visual_states`.
//! Cycles through each state automatically every 4 seconds; hover over the
//! capsule at any point to see the real compact/expanded morph.

#[cfg(windows)]
mod windows {
    use std::time::Instant;
    use verge_core::ports::{Metric, OverlayContent, OverlaySurface, StateTint, ToolGlyph};
    use verge_platform_windows::WindowsOverlaySurface;

    const CYCLE: std::time::Duration = std::time::Duration::from_secs(4);

    fn glyph(
        label: &str,
        mark: char,
        brand_color: (u8, u8, u8),
        state: StateTint,
        metric: Metric,
    ) -> ToolGlyph {
        ToolGlyph {
            sessions: vec![],
            session_summary: None,
            selected_session: None,
            label: label.to_string(),
            mark,
            brand_color,
            state,
            metric,
            dimmed: false,
            permission: if state == StateTint::Waiting {
                Some(verge_core::domain::PermissionRequest {
                    id: 1,
                    session_id: "fixture-session".into(),
                    tool: "Bash".into(),
                    detail: "{\n  \"command\": \"cargo test --workspace\"\n}".into(),
                    cwd: "D:\\example-project".into(),
                    created: Instant::now() - std::time::Duration::from_secs(20),
                    expires: Instant::now() + std::time::Duration::from_secs(100),
                })
            } else {
                None
            },
            session_count: Some(2),
            usage_windows: if let Metric::Fraction(f) = metric {
                vec![
                    verge_core::ports::UsageDetail {
                        label: "Current session".into(),
                        fraction: f,
                        reset: "Resets in 51 min".into(),
                        dimmed: false,
                    },
                    verge_core::ports::UsageDetail {
                        label: "All models".into(),
                        fraction: 0.07,
                        reset: "Resets in 2d".into(),
                        dimmed: false,
                    },
                ]
            } else {
                vec![]
            },
            reminder: None,
            activity_label: None,
            detail_lines: vec![format!("{label} — synthetic preview")],
        }
    }

    fn states() -> Vec<(&'static str, OverlayContent)> {
        vec![
            ("sessions", {
                let mut g = glyph(
                    "Claude",
                    '✳',
                    (217, 119, 87),
                    StateTint::Working,
                    Metric::Fraction(0.35),
                );
                g.session_summary = Some("Sessions · 1 context high".into());
                g.sessions = vec![
                    verge_core::ports::SessionDetail {
                        id: "fixture-one".into(),
                        title: "Example project".into(),
                        lines: vec![
                            "Model · Example model".into(),
                            "Context 82% · High".into(),
                            "Working".into(),
                            "Capacity · 200000 tokens".into(),
                        ],
                        priority: 3,
                    },
                    verge_core::ports::SessionDetail {
                        id: "fixture-two".into(),
                        title: "Second project".into(),
                        lines: vec![
                            "Model not reported".into(),
                            "Context not reported".into(),
                            "Activity not reported".into(),
                            "Capacity not reported".into(),
                        ],
                        priority: 1,
                    },
                ];
                OverlayContent {
                    glyphs: vec![g],
                    overflow_count: None,
                }
            }),
            ("idle", OverlayContent::default()),
            ("reminder", {
                let mut g = glyph(
                    "Claude",
                    '✳',
                    (217, 119, 87),
                    StateTint::Working,
                    Metric::Fraction(0.95),
                );
                g.reminder = Some("Current session: 95% used".into());
                g.activity_label = Some("Working".into());
                OverlayContent {
                    glyphs: vec![g],
                    overflow_count: None,
                }
            }),
            (
                "working",
                OverlayContent {
                    glyphs: vec![glyph(
                        "Claude",
                        '✳',
                        (217, 119, 87),
                        StateTint::Working,
                        Metric::Neutral,
                    )],
                    overflow_count: None,
                },
            ),
            (
                "permission",
                OverlayContent {
                    glyphs: vec![glyph(
                        "Claude",
                        '✳',
                        (217, 119, 87),
                        StateTint::Waiting,
                        Metric::Neutral,
                    )],
                    overflow_count: None,
                },
            ),
            (
                "usage ring (fraction)",
                OverlayContent {
                    glyphs: vec![glyph(
                        "Claude",
                        '✳',
                        (217, 119, 87),
                        StateTint::Neutral,
                        Metric::Fraction(0.73),
                    )],
                    overflow_count: None,
                },
            ),
            (
                "multiple active",
                OverlayContent {
                    glyphs: vec![
                        glyph(
                            "Claude",
                            '✳',
                            (217, 119, 87),
                            StateTint::Waiting,
                            Metric::Neutral,
                        ),
                        glyph(
                            "Claude",
                            '◈',
                            (217, 119, 87),
                            StateTint::Working,
                            Metric::Fraction(0.21),
                        ),
                        glyph(
                            "Claude",
                            '▲',
                            (217, 119, 87),
                            StateTint::Working,
                            Metric::None,
                        ),
                    ],
                    overflow_count: None,
                },
            ),
            (
                "overflow",
                OverlayContent {
                    glyphs: vec![
                        glyph(
                            "Claude",
                            '✳',
                            (217, 119, 87),
                            StateTint::Working,
                            Metric::Neutral,
                        ),
                        glyph(
                            "Claude",
                            '◈',
                            (217, 119, 87),
                            StateTint::Working,
                            Metric::None,
                        ),
                        glyph(
                            "Claude",
                            '▲',
                            (217, 119, 87),
                            StateTint::Working,
                            Metric::None,
                        ),
                        glyph(
                            "Claude",
                            '◫',
                            (217, 119, 87),
                            StateTint::Working,
                            Metric::None,
                        ),
                    ],
                    overflow_count: Some(2),
                },
            ),
            (
                "completed",
                OverlayContent {
                    glyphs: vec![glyph(
                        "Claude",
                        '✳',
                        (217, 119, 87),
                        StateTint::Completed,
                        Metric::Neutral,
                    )],
                    overflow_count: None,
                },
            ),
        ]
    }

    pub fn main() -> std::io::Result<()> {
        let all = states();
        // Isolated native interaction check; this service never reaches a real Claude session.
        if let Ok(path) = std::env::var("VERGE_PREVIEW_DECISION_FILE") {
            use std::sync::{Arc, Mutex};
            use verge_core::{
                domain::{PermissionDecision, PermissionRequest},
                ports::PermissionService,
            };
            struct PreviewDecision(Mutex<Option<PermissionRequest>>, String);
            impl PermissionService for PreviewDecision {
                fn pending(&self) -> Option<PermissionRequest> {
                    self.0.lock().unwrap().clone()
                }
                fn decide(&self, id: u64, session: &str, decision: PermissionDecision) -> bool {
                    let mut pending = self.0.lock().unwrap();
                    if !pending.as_ref().is_some_and(|r| {
                        r.id == id && r.session_id == session && Instant::now() < r.expires
                    }) {
                        return false;
                    }
                    if std::fs::write(&self.1, format!("{decision:?}")).is_err() {
                        return false;
                    }
                    *pending = None;
                    true
                }
            }
            let content = all
                .iter()
                .find(|(name, _)| *name == "permission")
                .unwrap()
                .1
                .clone();
            let service = Arc::new(PreviewDecision(
                Mutex::new(content.glyphs[0].permission.clone()),
                path,
            ));
            let source = service.clone();
            return WindowsOverlaySurface::new().run_with_permissions(
                move || {
                    if source.pending().is_some() {
                        content.clone()
                    } else {
                        OverlayContent::default()
                    }
                },
                service,
            );
        }
        let selected = std::env::args()
            .nth(1)
            .and_then(|name| all.iter().position(|(n, _)| *n == name));
        let start = std::time::Instant::now();
        // Fixture-only latency injection verifies that data I/O cannot stall native motion.
        let delay = std::env::var("VERGE_PREVIEW_DELAY_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        let first = std::cell::Cell::new(true);
        eprintln!("[visual_states] cycling {} states every {:?}; hover the capsule to check the expand/collapse morph", all.len(), CYCLE);
        for (name, _) in &all {
            eprintln!("  - {name}");
        }

        WindowsOverlaySurface::new().run(move || {
            if !first.replace(false) {
                std::thread::sleep(std::time::Duration::from_millis(delay));
            }
            let elapsed = start.elapsed();
            let index =
                selected.unwrap_or((elapsed.as_secs() / CYCLE.as_secs()) as usize % all.len());
            let mut content = all[index].1.clone();
            if std::env::args().any(|arg| arg == "--brand=ChatGPT") && all[index].0 == "sessions" {
                let glyph = &mut content.glyphs[0];
                glyph.label = "ChatGPT".into();
                glyph.brand_color = (235, 235, 235);
            }
            content
        })
    }
}
#[cfg(windows)]
fn main() -> std::io::Result<()> {
    windows::main()
}
#[cfg(not(windows))]
fn main() {
    eprintln!("This visual harness requires Windows");
}
