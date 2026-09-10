//! Shared Windows visual vocabulary. Dimensions and type sizes are DIPs;
//! layout converts them to physical pixels at the window's current DPI.
//! Tool colors belong to presentation data; Verge material and status light live here.

use std::time::Duration;

// ---- Color: Verge's own neutral material (never a tool's color, never a
// state color — see the scope note above) ----

/// Base material color — neutral, near-black, so tool identity and state
/// tints stay the only chromatic information on the surface.
pub const MATERIAL_RGB: (u8, u8, u8) = (3, 3, 4);
/// Dense material throughout; antialiasing alone softens the silhouette edge.
pub const MATERIAL_ALPHA_EDGE: f32 = 255.0;
pub const MATERIAL_ALPHA_INNER: f32 = 255.0;

pub const NEUTRAL_RING_RGB: (u8, u8, u8) = (255, 255, 255);
pub const NEUTRAL_RING_ALPHA: f32 = 0.14;

/// Usage colors follow the original frames: 21% green, 52% yellow, 73% orange.
/// The reference's prose says orange at 80%; its frame and UsageBand agree on 70%.
/// This colors an existing reading; it never estimates a limit or activity state.
pub fn usage_color(used: f32) -> (u8, u8, u8) {
    if used < 0.50 {
        (0, 255, 136)
    } else if used < 0.70 {
        (242, 255, 0)
    } else {
        (255, 63, 0)
    }
}

pub const TEXT_PRIMARY_RGB: (u8, u8, u8) = (235, 235, 235);
pub const TEXT_SECONDARY_RGB: (u8, u8, u8) = (157, 157, 160);
/// Multiplier applied to an aged/dimmed reading's alpha (material
/// treatment, never extra text — design spec §9).
pub const DIM_MULTIPLIER: f32 = 0.55;

// Instrument proportions, in DIPs. Round only at the physical raster boundary.
pub const SURFACE_SCALE: f32 = 0.9;
pub const FONT_FAMILY: &str = "Inter";
pub const TYPE_MICRO_PX: f32 = 14.0;
pub const TYPE_BODY_PX: f32 = 17.0;
pub const TYPE_TITLE_PX: f32 = 24.0;
pub const TYPE_NUMERAL_PX: f32 = 26.0;
pub const TYPE_OVERFLOW_PX: f32 = TYPE_MICRO_PX;
pub const GLYPH_DIAMETER: f32 = 28.0;
pub const RING_DIAMETER: f32 = 52.0;
pub const TRACK_THICKNESS: f32 = 7.0;
pub const RING_RADIUS: f32 = (RING_DIAMETER - TRACK_THICKNESS) / 2.0;
pub const RING_THICKNESS: f32 = 4.0;
pub const RING_LABEL_GAP: f32 = 10.0;
pub const NUMERAL_LINE_HEIGHT: f32 = 34.0;
pub const GLYPH_SLOT_HEIGHT: f32 = RING_DIAMETER + RING_LABEL_GAP + NUMERAL_LINE_HEIGHT;
pub const GLYPH_GAP_V: f32 = 24.0;
pub const GLYPH_COLUMN_WIDTH: f32 = 84.0;
pub const CORNER_RADIUS: f32 = 40.0;
pub const CAPSULE_PAD_V: f32 = 28.0;
pub const PAD_BOTTOM: f32 = 22.0;
pub const IDLE_WIDTH: f32 = 5.0;
pub const IDLE_HEIGHT: f32 = 80.0;
pub const IDLE_ALPHA: u8 = 235;
pub const TEXT_COLUMN_MAX_WIDTH: f32 = 308.0;
pub const CONNECTOR_WIDTH: f32 = 28.0;
pub const CONNECTOR_HALF_HEIGHT: f32 = 1.5;
pub const TEXT_COLUMN_PAD: f32 = 16.0;
pub const CONTENT_WIDTH: f32 = TEXT_COLUMN_MAX_WIDTH - CONNECTOR_WIDTH - 2.0 * TEXT_COLUMN_PAD;
pub const CARD_CORNER: f32 = 26.0;
pub const OVERFLOW_ROW_HEIGHT: f32 = 26.0;
pub const USAGE_BAR_HEIGHT: f32 = 6.0;
pub const BUTTON_RADIUS: f32 = 8.0;
pub const STATE_LIGHT_RADIUS: f32 = 25.0;
pub const STATE_LIGHT_SPREAD: f32 = 190.0;
pub const STATE_LIGHT_INTENSITY: f32 = 0.15;

// ---- Refresh / polling cadence ----

pub const TIMER_REFRESH: usize = 1;
pub const TIMER_HOVER: usize = 2;
pub const TIMER_ANIM: usize = 3;
pub const REFRESH_MS: u32 = 1000;
pub const HOVER_POLL_MS: u32 = 50;
/// Animation tick cadence while an expand/collapse is in flight — stops
/// itself once the target is reached, so this never runs while idle.
pub const ANIM_TICK_MS: u32 = 16;

/// Semantic motion durations (design spec §16 / `VERGE_DESIGN_SYSTEM.md`
/// §Motion). `Standard` is the only one with real behavior wired up today
/// (the compact/expanded morph); the rest are named here so a future
/// animated transition (a permission attention cue, a completion fade) has
/// an authoritative duration to reach for instead of inventing a new one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // named for completeness; only `Standard` has a wired-up transition today
pub enum MotionToken {
    Instant,
    Micro,
    Fast,
    Standard,
    Emphasized,
    Exit,
}

impl MotionToken {
    pub const fn duration(self) -> Duration {
        match self {
            MotionToken::Instant => Duration::ZERO,
            MotionToken::Micro => Duration::from_millis(80),
            MotionToken::Fast => Duration::from_millis(120),
            MotionToken::Standard => Duration::from_millis(240),
            MotionToken::Emphasized => Duration::from_millis(240),
            MotionToken::Exit => Duration::from_millis(140),
        }
    }
}

/// Ease-out cubic — "quiet, physical, confident" per the motion brief: fast
/// start, gentle settle, no overshoot.
pub fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// Inverse curve where the material joins the display edge.
pub const EDGE_FLARE: f32 = 44.0;
pub const HOVER_ENTER_MS: u64 = 100;
pub const HOVER_EXIT_MS: u64 = 220;
pub const INACTIVITY_SECONDS: u64 = 30;
pub const STATE_WORKING: (u8, u8, u8) = (215, 219, 224);
pub const STATE_WAITING: (u8, u8, u8) = (235, 202, 91);
pub const STATE_COMPLETED: (u8, u8, u8) = (44, 211, 100);
pub const STATE_STOPPED: (u8, u8, u8) = (255, 76, 68);

pub const WORKING_CYCLE_SECONDS: f32 = 12.0;
pub const WORKING_TRAVEL_DIP: f32 = 0.65;
pub const PERMISSION_SECONDS: f32 = 0.6;
pub const PERMISSION_TRAVEL_DIP: f32 = 1.2;

// Shared panel rhythm and interaction, in logical pixels / milliseconds.
pub const TITLE_LINE: f32 = 44.0;
pub const BODY_LINE: f32 = 24.0;
pub const USAGE_ROW: f32 = 78.0;
pub const DETAIL_HEIGHT: f32 = 168.0;
pub const PERMISSION_HEIGHT: f32 = 332.0;
pub const BUTTON_HEIGHT: f32 = 36.0;
pub const BUTTON_GAP: f32 = 10.0;
pub const PERMISSION_ARM_MS: u128 = 650;
pub const STATE_TICK_MS: u32 = 80;

pub const HEADER_MARK: f32 = 28.0;
pub const HEADER_IDENTITY_GAP: f32 = 10.0;
pub const TYPE_SESSION_PX: f32 = 15.0;
