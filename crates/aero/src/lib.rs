// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
pub mod atmosphere;
pub mod model;
pub mod table;

pub use model::{AeroModel, AeroOptions};
pub use table::{AeroCoeffs, AeroTable, Extrapolation};
