//! Reusable egui components shared by application views.

mod card;
mod color;
mod dropdown;
pub(crate) mod icons;
mod switch;

pub(crate) use card::{
    centered_label, config_card, config_card_with_header, config_row, config_row_with_divider,
    config_row_with_icon_and_divider, section_card,
};
pub(crate) use color::{lerp_color, with_alpha};
pub(crate) use dropdown::{DropdownOption, DropdownStyle, dropdown, dropdown_with_style};
pub(crate) use switch::{
    switch_row, switch_row_with_divider, toggle_switch, toggle_switch_with_id,
};
