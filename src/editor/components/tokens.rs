use iced::Color;

// Graphite workbench surfaces. Each structural layer has one deliberate tone.
pub const APP_BG: Color = Color::from_rgb(0.035, 0.035, 0.043);
pub const TOP_BAR_BG: Color = Color::from_rgb(0.055, 0.055, 0.064);
pub const PANEL_TITLE_BG: Color = Color::from_rgb(0.078, 0.078, 0.090);
pub const PANEL_BODY_BG: Color = Color::from_rgb(0.094, 0.094, 0.106);
pub const TOOLBAR_BG: Color = Color::from_rgb(0.135, 0.135, 0.149);
pub const INPUT_BG: Color = Color::from_rgb(0.035, 0.035, 0.043);
pub const BORDER: Color = Color::from_rgb(0.282, 0.282, 0.314);
pub const TEXT: Color = Color::from_rgb(0.980, 0.980, 0.980);
pub const MUTED: Color = Color::from_rgb(0.720, 0.720, 0.756);
pub const FAINT: Color = Color::from_rgb(0.550, 0.550, 0.590);
pub const CYAN: Color = Color::from_rgb(0.220, 0.827, 0.937);
pub const CYAN_SOFT: Color = Color::from_rgb(0.055, 0.165, 0.196);
pub const RED: Color = Color::from_rgb(0.973, 0.443, 0.443);
pub const YELLOW: Color = Color::from_rgb(0.984, 0.749, 0.141);

// Compatibility names for the drawing code. New component code uses the
// structural names above so hierarchy stays explicit.
pub const PANEL_BG: Color = PANEL_BODY_BG;
pub const PANEL_ALT: Color = TOOLBAR_BG;
pub const CONSOLE_RAIL: Color = TOOLBAR_BG;
pub const CONSOLE_WELL: Color = INPUT_BG;
pub const CONSOLE_LINE: Color = BORDER;
pub const CONSOLE_TEXT: Color = TEXT;
pub const CONSOLE_MUTED: Color = MUTED;
pub const CONSOLE_FAINT: Color = FAINT;

// Typography roles. A role, not a local layout, owns text size.
pub const BODY_SIZE: f32 = 12.0;
pub const CAPTION_SIZE: f32 = 11.0;
pub const OVERLINE_SIZE: f32 = 10.0;
pub const PANEL_TITLE_SIZE: f32 = 13.0;
pub const DIALOG_TITLE_SIZE: f32 = 15.0;
pub const TIMECODE_SIZE: f32 = 14.0;

// Density contract shared by every editor component.
pub const CONTROL_HEIGHT: f32 = 28.0;
pub const TOGGLE_SIZE: f32 = 18.0;
pub const ROW_HEIGHT: f32 = 30.0;
pub const DROPDOWN_ITEM_HEIGHT: f32 = CONTROL_HEIGHT;
pub const DROPDOWN_TEXT_LINE_HEIGHT: f32 = CONTROL_HEIGHT - (CONTROL_PAD_Y * 2.0);
pub const PANEL_TITLE_HEIGHT: f32 = 36.0;
pub const TOOLBAR_HEIGHT: f32 = 36.0;
pub const CONTROL_RADIUS: f32 = 3.0;
pub const CONTROL_PAD_X: f32 = 7.0;
pub const CONTROL_PAD_Y: f32 = 7.0;
pub const PANEL_PAD_X: f32 = 9.0;
pub const SECTION_GAP: f32 = 8.0;
