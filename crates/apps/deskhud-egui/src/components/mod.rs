//! Reusable egui components shared by application views.

mod card;
mod color;
mod dropdown;
mod icons;
mod switch;

pub(crate) use card::{centered_label, config_card, config_row};
pub(crate) use color::{lerp_color, with_alpha};
pub(crate) use dropdown::{DropdownOption, dropdown};
pub(crate) use switch::{switch_group, switch_row};
