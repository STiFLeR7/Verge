use std::time::{Duration, Instant};
use verge_core::ports::OverlayContent;

pub const WIDTH: u16 = 320;
pub const HEIGHT: u16 = 272;
pub const FOOTER: i16 = HEIGHT as i16 - 32;

/// Native selection stays attached to identities when the source reorders sessions.
pub struct Interaction {
    pub tool: Option<String>,
    pub session: Option<String>,
    pub touched: Instant,
    pub collapsed: bool,
}
impl Interaction {
    pub fn new(now: Instant) -> Self {
        Self {
            tool: None,
            session: None,
            touched: now,
            collapsed: false,
        }
    }
    pub fn touch(&mut self, now: Instant) {
        self.touched = now;
        self.collapsed = false;
    }
    pub fn tick(&mut self, now: Instant, content: &OverlayContent) {
        // Observation is not authority: waiting wakes the surface but does not enable decisions.
        let waiting = content
            .glyphs
            .iter()
            .any(|g| g.state == verge_core::ports::StateTint::Waiting);
        self.collapsed = !waiting && now.duration_since(self.touched) >= Duration::from_secs(30);
        if !content
            .glyphs
            .iter()
            .any(|g| Some(&g.label) == self.tool.as_ref())
        {
            self.tool = content.glyphs.first().map(|g| g.label.clone());
            self.session = None;
        }
        if let Some(g) = content
            .glyphs
            .iter()
            .find(|g| Some(&g.label) == self.tool.as_ref())
        {
            if !g
                .sessions
                .iter()
                .any(|s| Some(&s.id) == self.session.as_ref())
            {
                self.session = None;
            }
        }
    }
    pub fn click(&mut self, x: i16, y: i16, content: &OverlayContent, now: Instant) {
        if x < 0 || x >= WIDTH as i16 || y < 0 || y >= HEIGHT as i16 {
            return;
        }
        self.touch(now);
        if y < 32 {
            let count = content.glyphs.len();
            if count > 0 {
                let index = x as usize * count / WIDTH as usize;
                self.tool = Some(content.glyphs[index].label.clone());
                self.session = None;
            }
            return;
        }
        let Some(g) = content
            .glyphs
            .iter()
            .find(|g| Some(&g.label) == self.tool.as_ref())
        else {
            return;
        };
        if y < FOOTER {
            return;
        }
        let Some(id) = &self.session else {
            self.session = g.sessions.first().map(|s| s.id.clone());
            return;
        };
        // Back | previous | noninteractive count | next: same proportions as Windows.
        if x < WIDTH as i16 / 3 {
            self.session = None;
            return;
        }
        let Some(index) = g.sessions.iter().position(|s| &s.id == id) else {
            return;
        };
        let count = g.sessions.len();
        let next = if x < WIDTH as i16 / 2 {
            (index + count - 1) % count
        } else if x >= WIDTH as i16 * 5 / 6 {
            (index + 1) % count
        } else {
            return;
        };
        self.session = Some(g.sessions[next].id.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use verge_core::ports::{Metric, SessionDetail, StateTint, ToolGlyph};
    #[test]
    fn navigation_identity_and_inactivity_contract() {
        let now = Instant::now();
        let glyph = ToolGlyph {
            label: "ChatGPT".into(),
            mark: 'C',
            brand_color: (255, 255, 255),
            state: StateTint::Working,
            sessions: ["one", "two"]
                .iter()
                .map(|id| SessionDetail {
                    id: id.to_string(),
                    title: id.to_string(),
                    lines: vec![],
                    priority: 0,
                })
                .collect(),
            session_summary: None,
            selected_session: None,
            session_count: Some(2),
            permission: None,
            activity_label: None,
            metric: Metric::None,
            dimmed: false,
            detail_lines: vec![],
            usage_windows: vec![],
            reminder: None,
        };
        let mut content = OverlayContent {
            glyphs: vec![glyph],
            overflow_count: None,
        };
        let mut ui = Interaction::new(now);
        ui.tick(now, &content);
        ui.click(10, FOOTER, &content, now);
        assert_eq!(ui.session.as_deref(), Some("one"));
        ui.click(300, FOOTER, &content, now);
        assert_eq!(ui.session.as_deref(), Some("two"));
        ui.click(210, FOOTER, &content, now);
        assert_eq!(ui.session.as_deref(), Some("two"));
        content.glyphs[0].sessions.reverse();
        ui.tick(now, &content);
        assert_eq!(ui.session.as_deref(), Some("two"));
        ui.click(120, FOOTER, &content, now);
        assert_eq!(ui.session.as_deref(), Some("one"));
        ui.click(0, FOOTER, &content, now);
        assert!(ui.session.is_none());
        ui.tick(now + Duration::from_secs(29), &content);
        assert!(!ui.collapsed);
        ui.tick(now + Duration::from_secs(30), &content);
        assert!(ui.collapsed);
        content.glyphs[0].state = StateTint::Waiting;
        ui.tick(now + Duration::from_secs(31), &content);
        assert!(!ui.collapsed);
        content.glyphs.clear();
        ui.tick(now + Duration::from_secs(32), &content);
        ui.click(300, FOOTER, &content, now + Duration::from_secs(32));
        assert!(ui.tool.is_none());
    }
}
