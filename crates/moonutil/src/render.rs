// moon: The build system and package manager for MoonBit.
// Copyright (C) 2024 International Digital Economy Academy
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// For inquiries, you can contact us via e-mail at jichuruanjian@idea.edu.cn.

use ariadne::Fmt;
use serde::{Deserialize, Serialize};

use crate::common::{line_col_to_byte_idx, MOON_DOC_TEST_POSTFIX};

#[derive(Debug, Serialize, Deserialize)]
pub struct MooncDiagnostic {
    pub level: String,
    #[serde(alias = "loc")]
    pub location: Location,
    pub message: String,
    pub error_code: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Location {
    pub start: Position,
    pub end: Position,
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Position {
    pub line: usize,
    pub col: usize,
}

impl Position {
    pub fn calculate_offset(&self, content: &str) -> usize {
        let line_index = line_index::LineIndex::new(content);
        let byte_based_index =
            line_col_to_byte_idx(&line_index, self.line as u32 - 1, self.col as u32 - 1).unwrap();

        content
            .char_indices()
            .enumerate()
            .find(|(_, (byte_offset, _))| *byte_offset == byte_based_index)
            .map(|(i, _)| i)
            .unwrap_or(usize::from(line_index.len()))
    }
}

impl MooncDiagnostic {
    pub fn render(content: &str, use_fancy: bool) {
        match serde_json_lenient::from_str::<MooncDiagnostic>(content) {
            Ok(diagnostic) => {
                let (kind, color) = diagnostic.get_level_and_color();

                // for no-location diagnostic, like Missing main function in the main package(4067)
                if diagnostic.location.path.is_empty() {
                    eprintln!(
                        "{}",
                        format!(
                            "[{}] {}: {}",
                            diagnostic.error_code, kind, diagnostic.message
                        )
                        .fg(color)
                    );
                } else {
                    let source_file_path =
                        &if diagnostic.location.path.contains(MOON_DOC_TEST_POSTFIX) {
                            diagnostic.location.path.replace(MOON_DOC_TEST_POSTFIX, "")
                        } else {
                            diagnostic.location.path.clone()
                        };
                    let source_file = match std::fs::read_to_string(source_file_path) {
                        Ok(content) => content,
                        Err(_) => {
                            eprintln!(
                                "failed to read file `{}`, [{}] {}: {}",
                                source_file_path,
                                diagnostic.error_code,
                                diagnostic.level,
                                diagnostic.message
                            );
                            return;
                        }
                    };

                    let start_offset = diagnostic.location.start.calculate_offset(&source_file);
                    let end_offset = diagnostic.location.end.calculate_offset(&source_file);

                    let mut report_builder =
                        ariadne::Report::build(kind, source_file_path, start_offset)
                            .with_message(format!("[{}]", diagnostic.error_code).fg(color))
                            .with_label(
                                ariadne::Label::new((source_file_path, start_offset..end_offset))
                                    .with_message((&diagnostic.message).fg(color))
                                    .with_color(color),
                            );

                    if !use_fancy {
                        let config = ariadne::Config::default().with_color(false);
                        report_builder = report_builder.with_config(config);
                    }

                    report_builder
                        .finish()
                        .eprint((source_file_path, ariadne::Source::from(source_file)))
                        .unwrap();
                }
            }
            Err(_) => {
                eprintln!("{}", content);
            }
        }
    }

    fn get_level_and_color(&self) -> (ariadne::ReportKind, ariadne::Color) {
        if self.level == "error" {
            (ariadne::ReportKind::Error, ariadne::Color::Red)
        } else if self.level == "warning" {
            (ariadne::ReportKind::Warning, ariadne::Color::BrightYellow)
        } else {
            (ariadne::ReportKind::Advice, ariadne::Color::Blue)
        }
    }
}
