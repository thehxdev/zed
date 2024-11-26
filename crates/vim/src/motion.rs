use editor::{
    display_map::{DisplayRow, DisplaySnapshot, FoldPoint, ToDisplayPoint},
    movement::{
        self, find_boundary, find_preceding_boundary_display_point, FindRange, TextLayoutDetails,
    },
    scroll::Autoscroll,
    Anchor, Bias, DisplayPoint, Editor, RowExt, ToOffset,
};
use gpui::{actions, impl_actions, px, ViewContext};
use language::{CharKind, Point, Selection, SelectionGoal};
use multi_buffer::MultiBufferRow;
use serde::Deserialize;
use std::ops::Range;

use crate::{
    normal::mark,
    state::{Mode, Operator},
    surrounds::SurroundsType,
    Vim,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Motion {
    Left,
    Backspace,
    Down {
        display_lines: bool,
    },
    Up {
        display_lines: bool,
    },
    Right,
    Space,
    NextWordStart {
        ignore_punctuation: bool,
    },
    NextWordEnd {
        ignore_punctuation: bool,
    },
    PreviousWordStart {
        ignore_punctuation: bool,
    },
    PreviousWordEnd {
        ignore_punctuation: bool,
    },
    NextSubwordStart {
        ignore_punctuation: bool,
    },
    NextSubwordEnd {
        ignore_punctuation: bool,
    },
    PreviousSubwordStart {
        ignore_punctuation: bool,
    },
    PreviousSubwordEnd {
        ignore_punctuation: bool,
    },
    FirstNonWhitespace {
        display_lines: bool,
    },
    CurrentLine,
    StartOfLine {
        display_lines: bool,
    },
    EndOfLine {
        display_lines: bool,
    },
    SentenceBackward,
    SentenceForward,
    StartOfParagraph,
    EndOfParagraph,
    StartOfDocument,
    EndOfDocument,
    Matching,
    UnmatchedForward {
        char: char,
    },
    UnmatchedBackward {
        char: char,
    },
    FindForward {
        before: bool,
        char: char,
        mode: FindRange,
        smartcase: bool,
    },
    FindBackward {
        after: bool,
        char: char,
        mode: FindRange,
        smartcase: bool,
    },
    RepeatFind {
        last_find: Box<Motion>,
    },
    RepeatFindReversed {
        last_find: Box<Motion>,
    },
    NextLineStart,
    PreviousLineStart,
    StartOfLineDownward,
    EndOfLineDownward,
    GoToColumn,
    WindowTop,
    WindowMiddle,
    WindowBottom,

    // we don't have a good way to run a search synchronously, so
    // we handle search motions by running the search async and then
    // calling back into motion with this
    ZedSearchResult {
        prior_selections: Vec<Range<Anchor>>,
        new_selections: Vec<Range<Anchor>>,
    },
    Jump {
        anchor: Anchor,
        line: bool,
    },
}

#[derive(Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct NextWordStart {
    #[serde(default)]
    ignore_punctuation: bool,
}

#[derive(Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct NextWordEnd {
    #[serde(default)]
    ignore_punctuation: bool,
}

#[derive(Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct PreviousWordStart {
    #[serde(default)]
    ignore_punctuation: bool,
}

#[derive(Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct PreviousWordEnd {
    #[serde(default)]
    ignore_punctuation: bool,
}

#[derive(Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NextSubwordStart {
    #[serde(default)]
    pub(crate) ignore_punctuation: bool,
}

#[derive(Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NextSubwordEnd {
    #[serde(default)]
    pub(crate) ignore_punctuation: bool,
}

#[derive(Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreviousSubwordStart {
    #[serde(default)]
    pub(crate) ignore_punctuation: bool,
}

#[derive(Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreviousSubwordEnd {
    #[serde(default)]
    pub(crate) ignore_punctuation: bool,
}

#[derive(Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Up {
    #[serde(default)]
    pub(crate) display_lines: bool,
}

#[derive(Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Down {
    #[serde(default)]
    pub(crate) display_lines: bool,
}

#[derive(Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct FirstNonWhitespace {
    #[serde(default)]
    display_lines: bool,
}

#[derive(Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct EndOfLine {
    #[serde(default)]
    display_lines: bool,
}

#[derive(Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StartOfLine {
    #[serde(default)]
    pub(crate) display_lines: bool,
}

#[derive(Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct UnmatchedForward {
    #[serde(default)]
    char: char,
}

#[derive(Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct UnmatchedBackward {
    #[serde(default)]
    char: char,
}

impl_actions!(
    vim,
    [
        StartOfLine,
        EndOfLine,
        FirstNonWhitespace,
        Down,
        Up,
        NextWordStart,
        NextWordEnd,
        PreviousWordStart,
        PreviousWordEnd,
        NextSubwordStart,
        NextSubwordEnd,
        PreviousSubwordStart,
        PreviousSubwordEnd,
        UnmatchedForward,
        UnmatchedBackward
    ]
);

actions!(
    vim,
    [
        Left,
        Backspace,
        Right,
        Space,
        CurrentLine,
        SentenceForward,
        SentenceBackward,
        StartOfParagraph,
        EndOfParagraph,
        StartOfDocument,
        EndOfDocument,
        Matching,
        NextLineStart,
        PreviousLineStart,
        StartOfLineDownward,
        EndOfLineDownward,
        GoToColumn,
        RepeatFind,
        RepeatFindReversed,
        WindowTop,
        WindowMiddle,
        WindowBottom,
    ]
);

pub fn register(editor: &mut Editor, cx: &mut ViewContext<Vim>) {
    Vim::action(editor, cx, |vim, _: &Left, cx| vim.motion(Motion::Left, cx));
    Vim::action(editor, cx, |vim, _: &Backspace, cx| {
        vim.motion(Motion::Backspace, cx)
    });
    Vim::action(editor, cx, |vim, action: &Down, cx| {
        vim.motion(
            Motion::Down {
                display_lines: action.display_lines,
            },
            cx,
        )
    });
    Vim::action(editor, cx, |vim, action: &Up, cx| {
        vim.motion(
            Motion::Up {
                display_lines: action.display_lines,
            },
            cx,
        )
    });
    Vim::action(editor, cx, |vim, _: &Right, cx| {
        vim.motion(Motion::Right, cx)
    });
    Vim::action(editor, cx, |vim, _: &Space, cx| {
        vim.motion(Motion::Space, cx)
    });
    Vim::action(editor, cx, |vim, action: &FirstNonWhitespace, cx| {
        vim.motion(
            Motion::FirstNonWhitespace {
                display_lines: action.display_lines,
            },
            cx,
        )
    });
    Vim::action(editor, cx, |vim, action: &StartOfLine, cx| {
        vim.motion(
            Motion::StartOfLine {
                display_lines: action.display_lines,
            },
            cx,
        )
    });
    Vim::action(editor, cx, |vim, action: &EndOfLine, cx| {
        vim.motion(
            Motion::EndOfLine {
                display_lines: action.display_lines,
            },
            cx,
        )
    });
    Vim::action(editor, cx, |vim, _: &CurrentLine, cx| {
        vim.motion(Motion::CurrentLine, cx)
    });
    Vim::action(editor, cx, |vim, _: &StartOfParagraph, cx| {
        vim.motion(Motion::StartOfParagraph, cx)
    });
    Vim::action(editor, cx, |vim, _: &EndOfParagraph, cx| {
        vim.motion(Motion::EndOfParagraph, cx)
    });

    Vim::action(editor, cx, |vim, _: &SentenceForward, cx| {
        vim.motion(Motion::SentenceForward, cx)
    });
    Vim::action(editor, cx, |vim, _: &SentenceBackward, cx| {
        vim.motion(Motion::SentenceBackward, cx)
    });
    Vim::action(editor, cx, |vim, _: &StartOfDocument, cx| {
        vim.motion(Motion::StartOfDocument, cx)
    });
    Vim::action(editor, cx, |vim, _: &EndOfDocument, cx| {
        vim.motion(Motion::EndOfDocument, cx)
    });
    Vim::action(editor, cx, |vim, _: &Matching, cx| {
        vim.motion(Motion::Matching, cx)
    });
    Vim::action(
        editor,
        cx,
        |vim, &UnmatchedForward { char }: &UnmatchedForward, cx| {
            vim.motion(Motion::UnmatchedForward { char }, cx)
        },
    );
    Vim::action(
        editor,
        cx,
        |vim, &UnmatchedBackward { char }: &UnmatchedBackward, cx| {
            vim.motion(Motion::UnmatchedBackward { char }, cx)
        },
    );
    Vim::action(
        editor,
        cx,
        |vim, &NextWordStart { ignore_punctuation }: &NextWordStart, cx| {
            vim.motion(Motion::NextWordStart { ignore_punctuation }, cx)
        },
    );
    Vim::action(
        editor,
        cx,
        |vim, &NextWordEnd { ignore_punctuation }: &NextWordEnd, cx| {
            vim.motion(Motion::NextWordEnd { ignore_punctuation }, cx)
        },
    );
    Vim::action(
        editor,
        cx,
        |vim, &PreviousWordStart { ignore_punctuation }: &PreviousWordStart, cx| {
            vim.motion(Motion::PreviousWordStart { ignore_punctuation }, cx)
        },
    );
    Vim::action(
        editor,
        cx,
        |vim, &PreviousWordEnd { ignore_punctuation }, cx| {
            vim.motion(Motion::PreviousWordEnd { ignore_punctuation }, cx)
        },
    );
    Vim::action(
        editor,
        cx,
        |vim, &NextSubwordStart { ignore_punctuation }: &NextSubwordStart, cx| {
            vim.motion(Motion::NextSubwordStart { ignore_punctuation }, cx)
        },
    );
    Vim::action(
        editor,
        cx,
        |vim, &NextSubwordEnd { ignore_punctuation }: &NextSubwordEnd, cx| {
            vim.motion(Motion::NextSubwordEnd { ignore_punctuation }, cx)
        },
    );
    Vim::action(
        editor,
        cx,
        |vim, &PreviousSubwordStart { ignore_punctuation }: &PreviousSubwordStart, cx| {
            vim.motion(Motion::PreviousSubwordStart { ignore_punctuation }, cx)
        },
    );
    Vim::action(
        editor,
        cx,
        |vim, &PreviousSubwordEnd { ignore_punctuation }, cx| {
            vim.motion(Motion::PreviousSubwordEnd { ignore_punctuation }, cx)
        },
    );
    Vim::action(editor, cx, |vim, &NextLineStart, cx| {
        vim.motion(Motion::NextLineStart, cx)
    });
    Vim::action(editor, cx, |vim, &PreviousLineStart, cx| {
        vim.motion(Motion::PreviousLineStart, cx)
    });
    Vim::action(editor, cx, |vim, &StartOfLineDownward, cx| {
        vim.motion(Motion::StartOfLineDownward, cx)
    });
    Vim::action(editor, cx, |vim, &EndOfLineDownward, cx| {
        vim.motion(Motion::EndOfLineDownward, cx)
    });
    Vim::action(editor, cx, |vim, &GoToColumn, cx| {
        vim.motion(Motion::GoToColumn, cx)
    });

    Vim::action(editor, cx, |vim, _: &RepeatFind, cx| {
        if let Some(last_find) = Vim::globals(cx).last_find.clone().map(Box::new) {
            vim.motion(Motion::RepeatFind { last_find }, cx);
        }
    });

    Vim::action(editor, cx, |vim, _: &RepeatFindReversed, cx| {
        if let Some(last_find) = Vim::globals(cx).last_find.clone().map(Box::new) {
            vim.motion(Motion::RepeatFindReversed { last_find }, cx);
        }
    });
    Vim::action(editor, cx, |vim, &WindowTop, cx| {
        vim.motion(Motion::WindowTop, cx)
    });
    Vim::action(editor, cx, |vim, &WindowMiddle, cx| {
        vim.motion(Motion::WindowMiddle, cx)
    });
    Vim::action(editor, cx, |vim, &WindowBottom, cx| {
        vim.motion(Motion::WindowBottom, cx)
    });
}

impl Vim {
    pub(crate) fn search_motion(&mut self, m: Motion, cx: &mut ViewContext<Self>) {
        if let Motion::ZedSearchResult {
            prior_selections, ..
        } = &m
        {
            match self.mode {
                Mode::Visual | Mode::VisualLine | Mode::VisualBlock => {
                    if !prior_selections.is_empty() {
                        self.update_editor(cx, |_, editor, cx| {
                            editor.change_selections(Some(Autoscroll::fit()), cx, |s| {
                                s.select_ranges(prior_selections.iter().cloned())
                            })
                        });
                    }
                }
                Mode::Normal | Mode::Replace | Mode::Insert => {
                    if self.active_operator().is_none() {
                        return;
                    }
                }
            }
        }

        self.motion(m, cx)
    }

    pub(crate) fn motion(&mut self, motion: Motion, cx: &mut ViewContext<Self>) {
        if let Some(Operator::FindForward { .. }) | Some(Operator::FindBackward { .. }) =
            self.active_operator()
        {
            self.pop_operator(cx);
        }

        let count = self.take_count(cx);
        let active_operator = self.active_operator();
        let mut waiting_operator: Option<Operator> = None;
        match self.mode {
            Mode::Normal | Mode::Replace | Mode::Insert => {
                if active_operator == Some(Operator::AddSurrounds { target: None }) {
                    waiting_operator = Some(Operator::AddSurrounds {
                        target: Some(SurroundsType::Motion(motion)),
                    });
                } else {
                    self.normal_motion(motion.clone(), active_operator.clone(), count, cx)
                }
            }
            Mode::Visual | Mode::VisualLine | Mode::VisualBlock => {
                self.visual_motion(motion.clone(), count, cx)
            }
        }
        self.clear_operator(cx);
        if let Some(operator) = waiting_operator {
            self.push_operator(operator, cx);
            self.pre_count = count
        }
    }
}

// Motion handling is specified here:
// https://github.com/vim/vim/blob/master/runtime/doc/motion.txt
impl Motion {
    pub fn linewise(&self) -> bool {
        use Motion::*;
        match self {
            Down { .. }
            | Up { .. }
            | StartOfDocument
            | EndOfDocument
            | CurrentLine
            | NextLineStart
            | PreviousLineStart
            | StartOfLineDownward
            | SentenceBackward
            | SentenceForward
            | StartOfParagraph
            | EndOfParagraph
            | WindowTop
            | WindowMiddle
            | WindowBottom
            | Jump { line: true, .. } => true,
            EndOfLine { .. }
            | Matching
            | UnmatchedForward { .. }
            | UnmatchedBackward { .. }
            | FindForward { .. }
            | Left
            | Backspace
            | Right
            | Space
            | StartOfLine { .. }
            | EndOfLineDownward
            | GoToColumn
            | NextWordStart { .. }
            | NextWordEnd { .. }
            | PreviousWordStart { .. }
            | PreviousWordEnd { .. }
            | NextSubwordStart { .. }
            | NextSubwordEnd { .. }
            | PreviousSubwordStart { .. }
            | PreviousSubwordEnd { .. }
            | FirstNonWhitespace { .. }
            | FindBackward { .. }
            | RepeatFind { .. }
            | RepeatFindReversed { .. }
            | Jump { line: false, .. }
            | ZedSearchResult { .. } => false,
        }
    }

    pub fn infallible(&self) -> bool {
        use Motion::*;
        match self {
            StartOfDocument | EndOfDocument | CurrentLine => true,
            Down { .. }
            | Up { .. }
            | EndOfLine { .. }
            | Matching
            | UnmatchedForward { .. }
            | UnmatchedBackward { .. }
            | FindForward { .. }
            | RepeatFind { .. }
            | Left
            | Backspace
            | Right
            | Space
            | StartOfLine { .. }
            | StartOfParagraph
            | EndOfParagraph
            | SentenceBackward
            | SentenceForward
            | StartOfLineDownward
            | EndOfLineDownward
            | GoToColumn
            | NextWordStart { .. }
            | NextWordEnd { .. }
            | PreviousWordStart { .. }
            | PreviousWordEnd { .. }
            | NextSubwordStart { .. }
            | NextSubwordEnd { .. }
            | PreviousSubwordStart { .. }
            | PreviousSubwordEnd { .. }
            | FirstNonWhitespace { .. }
            | FindBackward { .. }
            | RepeatFindReversed { .. }
            | WindowTop
            | WindowMiddle
            | WindowBottom
            | NextLineStart
            | PreviousLineStart
            | ZedSearchResult { .. }
            | Jump { .. } => false,
        }
    }

    pub fn inclusive(&self) -> bool {
        use Motion::*;
        match self {
            Down { .. }
            | Up { .. }
            | StartOfDocument
            | EndOfDocument
            | CurrentLine
            | EndOfLine { .. }
            | EndOfLineDownward
            | Matching
            | UnmatchedForward { .. }
            | UnmatchedBackward { .. }
            | FindForward { .. }
            | WindowTop
            | WindowMiddle
            | WindowBottom
            | NextWordEnd { .. }
            | PreviousWordEnd { .. }
            | NextSubwordEnd { .. }
            | PreviousSubwordEnd { .. }
            | NextLineStart
            | PreviousLineStart => true,
            Left
            | Backspace
            | Right
            | Space
            | StartOfLine { .. }
            | StartOfLineDownward
            | StartOfParagraph
            | EndOfParagraph
            | SentenceBackward
            | SentenceForward
            | GoToColumn
            | NextWordStart { .. }
            | PreviousWordStart { .. }
            | NextSubwordStart { .. }
            | PreviousSubwordStart { .. }
            | FirstNonWhitespace { .. }
            | FindBackward { .. }
            | Jump { .. }
            | ZedSearchResult { .. } => false,
            RepeatFind { last_find: motion } | RepeatFindReversed { last_find: motion } => {
                motion.inclusive()
            }
        }
    }

    pub fn move_point(
        &self,
        map: &DisplaySnapshot,
        point: DisplayPoint,
        goal: SelectionGoal,
        maybe_times: Option<usize>,
        text_layout_details: &TextLayoutDetails,
    ) -> Option<(DisplayPoint, SelectionGoal)> {
        let times = maybe_times.unwrap_or(1);
        use Motion::*;
        let infallible = self.infallible();
        let (new_point, goal) = match self {
            Left => (left(map, point, times), SelectionGoal::None),
            Backspace => (backspace(map, point, times), SelectionGoal::None),
            Down {
                display_lines: false,
            } => up_down_buffer_rows(map, point, goal, times as isize, text_layout_details),
            Down {
                display_lines: true,
            } => down_display(map, point, goal, times, text_layout_details),
            Up {
                display_lines: false,
            } => up_down_buffer_rows(map, point, goal, 0 - times as isize, text_layout_details),
            Up {
                display_lines: true,
            } => up_display(map, point, goal, times, text_layout_details),
            Right => (right(map, point, times), SelectionGoal::None),
            Space => (space(map, point, times), SelectionGoal::None),
            NextWordStart { ignore_punctuation } => (
                next_word_start(map, point, *ignore_punctuation, times),
                SelectionGoal::None,
            ),
            NextWordEnd { ignore_punctuation } => (
                next_word_end(map, point, *ignore_punctuation, times, true),
                SelectionGoal::None,
            ),
            PreviousWordStart { ignore_punctuation } => (
                previous_word_start(map, point, *ignore_punctuation, times),
                SelectionGoal::None,
            ),
            PreviousWordEnd { ignore_punctuation } => (
                previous_word_end(map, point, *ignore_punctuation, times),
                SelectionGoal::None,
            ),
            NextSubwordStart { ignore_punctuation } => (
                next_subword_start(map, point, *ignore_punctuation, times),
                SelectionGoal::None,
            ),
            NextSubwordEnd { ignore_punctuation } => (
                next_subword_end(map, point, *ignore_punctuation, times, true),
                SelectionGoal::None,
            ),
            PreviousSubwordStart { ignore_punctuation } => (
                previous_subword_start(map, point, *ignore_punctuation, times),
                SelectionGoal::None,
            ),
            PreviousSubwordEnd { ignore_punctuation } => (
                previous_subword_end(map, point, *ignore_punctuation, times),
                SelectionGoal::None,
            ),
            FirstNonWhitespace { display_lines } => (
                first_non_whitespace(map, *display_lines, point),
                SelectionGoal::None,
            ),
            StartOfLine { display_lines } => (
                start_of_line(map, *display_lines, point),
                SelectionGoal::None,
            ),
            EndOfLine { display_lines } => (
                end_of_line(map, *display_lines, point, times),
                SelectionGoal::None,
            ),
            SentenceBackward => (sentence_backwards(map, point, times), SelectionGoal::None),
            SentenceForward => (sentence_forwards(map, point, times), SelectionGoal::None),
            StartOfParagraph => (
                movement::start_of_paragraph(map, point, times),
                SelectionGoal::None,
            ),
            EndOfParagraph => (
                map.clip_at_line_end(movement::end_of_paragraph(map, point, times)),
                SelectionGoal::None,
            ),
            CurrentLine => (next_line_end(map, point, times), SelectionGoal::None),
            StartOfDocument => (start_of_document(map, point, times), SelectionGoal::None),
            EndOfDocument => (
                end_of_document(map, point, maybe_times),
                SelectionGoal::None,
            ),
            Matching => (matching(map, point), SelectionGoal::None),
            UnmatchedForward { char } => (
                unmatched_forward(map, point, *char, times),
                SelectionGoal::None,
            ),
            UnmatchedBackward { char } => (
                unmatched_backward(map, point, *char, times),
                SelectionGoal::None,
            ),
            // t f
            FindForward {
                before,
                char,
                mode,
                smartcase,
            } => {
                return find_forward(map, point, *before, *char, times, *mode, *smartcase)
                    .map(|new_point| (new_point, SelectionGoal::None))
            }
            // T F
            FindBackward {
                after,
                char,
                mode,
                smartcase,
            } => (
                find_backward(map, point, *after, *char, times, *mode, *smartcase),
                SelectionGoal::None,
            ),
            // ; -- repeat the last find done with t, f, T, F
            RepeatFind { last_find } => match **last_find {
                Motion::FindForward {
                    before,
                    char,
                    mode,
                    smartcase,
                } => {
                    let mut new_point =
                        find_forward(map, point, before, char, times, mode, smartcase);
                    if new_point == Some(point) {
                        new_point =
                            find_forward(map, point, before, char, times + 1, mode, smartcase);
                    }

                    return new_point.map(|new_point| (new_point, SelectionGoal::None));
                }

                Motion::FindBackward {
                    after,
                    char,
                    mode,
                    smartcase,
                } => {
                    let mut new_point =
                        find_backward(map, point, after, char, times, mode, smartcase);
                    if new_point == point {
                        new_point =
                            find_backward(map, point, after, char, times + 1, mode, smartcase);
                    }

                    (new_point, SelectionGoal::None)
                }
                _ => return None,
            },
            // , -- repeat the last find done with t, f, T, F, in opposite direction
            RepeatFindReversed { last_find } => match **last_find {
                Motion::FindForward {
                    before,
                    char,
                    mode,
                    smartcase,
                } => {
                    let mut new_point =
                        find_backward(map, point, before, char, times, mode, smartcase);
                    if new_point == point {
                        new_point =
                            find_backward(map, point, before, char, times + 1, mode, smartcase);
                    }

                    (new_point, SelectionGoal::None)
                }

                Motion::FindBackward {
                    after,
                    char,
                    mode,
                    smartcase,
                } => {
                    let mut new_point =
                        find_forward(map, point, after, char, times, mode, smartcase);
                    if new_point == Some(point) {
                        new_point =
                            find_forward(map, point, after, char, times + 1, mode, smartcase);
                    }

                    return new_point.map(|new_point| (new_point, SelectionGoal::None));
                }
                _ => return None,
            },
            NextLineStart => (next_line_start(map, point, times), SelectionGoal::None),
            PreviousLineStart => (previous_line_start(map, point, times), SelectionGoal::None),
            StartOfLineDownward => (next_line_start(map, point, times - 1), SelectionGoal::None),
            EndOfLineDownward => (last_non_whitespace(map, point, times), SelectionGoal::None),
            GoToColumn => (go_to_column(map, point, times), SelectionGoal::None),
            WindowTop => window_top(map, point, text_layout_details, times - 1),
            WindowMiddle => window_middle(map, point, text_layout_details),
            WindowBottom => window_bottom(map, point, text_layout_details, times - 1),
            Jump { line, anchor } => mark::jump_motion(map, *anchor, *line),
            ZedSearchResult { new_selections, .. } => {
                // There will be only one selection, as
                // Search::SelectNextMatch selects a single match.
                if let Some(new_selection) = new_selections.first() {
                    (
                        new_selection.start.to_display_point(map),
                        SelectionGoal::None,
                    )
                } else {
                    return None;
                }
            }
        };

        (new_point != point || infallible).then_some((new_point, goal))
    }

    // Get the range value after self is applied to the specified selection.
    pub fn range(
        &self,
        map: &DisplaySnapshot,
        selection: Selection<DisplayPoint>,
        times: Option<usize>,
        expand_to_surrounding_newline: bool,
        text_layout_details: &TextLayoutDetails,
    ) -> Option<Range<DisplayPoint>> {
        if let Motion::ZedSearchResult {
            prior_selections,
            new_selections,
        } = self
        {
            if let Some((prior_selection, new_selection)) =
                prior_selections.first().zip(new_selections.first())
            {
                let start = prior_selection
                    .start
                    .to_display_point(map)
                    .min(new_selection.start.to_display_point(map));
                let end = new_selection
                    .end
                    .to_display_point(map)
                    .max(prior_selection.end.to_display_point(map));

                if start < end {
                    return Some(start..end);
                } else {
                    return Some(end..start);
                }
            } else {
                return None;
            }
        }

        if let Some((new_head, goal)) = self.move_point(
            map,
            selection.head(),
            selection.goal,
            times,
            text_layout_details,
        ) {
            let mut selection = selection.clone();
            selection.set_head(new_head, goal);

            if self.linewise() {
                selection.start = map.prev_line_boundary(selection.start.to_point(map)).1;

                if expand_to_surrounding_newline {
                    if selection.end.row() < map.max_point().row() {
                        *selection.end.row_mut() += 1;
                        *selection.end.column_mut() = 0;
                        selection.end = map.clip_point(selection.end, Bias::Right);
                        // Don't reset the end here
                        return Some(selection.start..selection.end);
                    } else if selection.start.row().0 > 0 {
                        *selection.start.row_mut() -= 1;
                        *selection.start.column_mut() = map.line_len(selection.start.row());
                        selection.start = map.clip_point(selection.start, Bias::Left);
                    }
                }

                selection.end = map.next_line_boundary(selection.end.to_point(map)).1;
            } else {
                // Another special case: When using the "w" motion in combination with an
                // operator and the last word moved over is at the end of a line, the end of
                // that word becomes the end of the operated text, not the first word in the
                // next line.
                if let Motion::NextWordStart {
                    ignore_punctuation: _,
                } = self
                {
                    let start_row = MultiBufferRow(selection.start.to_point(map).row);
                    if selection.end.to_point(map).row > start_row.0 {
                        selection.end =
                            Point::new(start_row.0, map.buffer_snapshot.line_len(start_row))
                                .to_display_point(map)
                    }
                }

                // If the motion is exclusive and the end of the motion is in column 1, the
                // end of the motion is moved to the end of the previous line and the motion
                // becomes inclusive. Example: "}" moves to the first line after a paragraph,
                // but "d}" will not include that line.
                let mut inclusive = self.inclusive();
                let start_point = selection.start.to_point(map);
                let mut end_point = selection.end.to_point(map);

                // DisplayPoint

                if !inclusive
                    && self != &Motion::Backspace
                    && end_point.row > start_point.row
                    && end_point.column == 0
                {
                    inclusive = true;
                    end_point.row -= 1;
                    end_point.column = 0;
                    selection.end = map.clip_point(map.next_line_boundary(end_point).1, Bias::Left);
                }

                if inclusive && selection.end.column() < map.line_len(selection.end.row()) {
                    selection.end = movement::saturating_right(map, selection.end)
                }
            }
            Some(selection.start..selection.end)
        } else {
            None
        }
    }

    // Expands a selection using self for an operator
    pub fn expand_selection(
        &self,
        map: &DisplaySnapshot,
        selection: &mut Selection<DisplayPoint>,
        times: Option<usize>,
        expand_to_surrounding_newline: bool,
        text_layout_details: &TextLayoutDetails,
    ) -> bool {
        if let Some(range) = self.range(
            map,
            selection.clone(),
            times,
            expand_to_surrounding_newline,
            text_layout_details,
        ) {
            selection.start = range.start;
            selection.end = range.end;
            true
        } else {
            false
        }
    }
}

fn left(map: &DisplaySnapshot, mut point: DisplayPoint, times: usize) -> DisplayPoint {
    for _ in 0..times {
        point = movement::saturating_left(map, point);
        if point.column() == 0 {
            break;
        }
    }
    point
}

pub(crate) fn backspace(
    map: &DisplaySnapshot,
    mut point: DisplayPoint,
    times: usize,
) -> DisplayPoint {
    for _ in 0..times {
        point = movement::left(map, point);
        if point.is_zero() {
            break;
        }
    }
    point
}

fn space(map: &DisplaySnapshot, mut point: DisplayPoint, times: usize) -> DisplayPoint {
    for _ in 0..times {
        point = wrapping_right(map, point);
        if point == map.max_point() {
            break;
        }
    }
    point
}

fn wrapping_right(map: &DisplaySnapshot, mut point: DisplayPoint) -> DisplayPoint {
    let max_column = map.line_len(point.row()).saturating_sub(1);
    if point.column() < max_column {
        *point.column_mut() += 1;
    } else if point.row() < map.max_point().row() {
        *point.row_mut() += 1;
        *point.column_mut() = 0;
    }
    point
}

pub(crate) fn start_of_relative_buffer_row(
    map: &DisplaySnapshot,
    point: DisplayPoint,
    times: isize,
) -> DisplayPoint {
    let start = map.display_point_to_fold_point(point, Bias::Left);
    let target = start.row() as isize + times;
    let new_row = (target.max(0) as u32).min(map.fold_snapshot.max_point().row());

    map.clip_point(
        map.fold_point_to_display_point(
            map.fold_snapshot
                .clip_point(FoldPoint::new(new_row, 0), Bias::Right),
        ),
        Bias::Right,
    )
}

fn up_down_buffer_rows(
    map: &DisplaySnapshot,
    point: DisplayPoint,
    mut goal: SelectionGoal,
    times: isize,
    text_layout_details: &TextLayoutDetails,
) -> (DisplayPoint, SelectionGoal) {
    let start = map.display_point_to_fold_point(point, Bias::Left);
    let begin_folded_line = map.fold_point_to_display_point(
        map.fold_snapshot
            .clip_point(FoldPoint::new(start.row(), 0), Bias::Left),
    );
    let select_nth_wrapped_row = point.row().0 - begin_folded_line.row().0;

    let (goal_wrap, goal_x) = match goal {
        SelectionGoal::WrappedHorizontalPosition((row, x)) => (row, x),
        SelectionGoal::HorizontalRange { end, .. } => (select_nth_wrapped_row, end),
        SelectionGoal::HorizontalPosition(x) => (select_nth_wrapped_row, x),
        _ => {
            let x = map.x_for_display_point(point, text_layout_details);
            goal = SelectionGoal::WrappedHorizontalPosition((select_nth_wrapped_row, x.0));
            (select_nth_wrapped_row, x.0)
        }
    };

    let target = start.row() as isize + times;
    let new_row = (target.max(0) as u32).min(map.fold_snapshot.max_point().row());

    let mut begin_folded_line = map.fold_point_to_display_point(
        map.fold_snapshot
            .clip_point(FoldPoint::new(new_row, 0), Bias::Left),
    );

    let mut i = 0;
    while i < goal_wrap && begin_folded_line.row() < map.max_point().row() {
        let next_folded_line = DisplayPoint::new(begin_folded_line.row().next_row(), 0);
        if map
            .display_point_to_fold_point(next_folded_line, Bias::Right)
            .row()
            == new_row
        {
            i += 1;
            begin_folded_line = next_folded_line;
        } else {
            break;
        }
    }

    let new_col = if i == goal_wrap {
        map.display_column_for_x(begin_folded_line.row(), px(goal_x), text_layout_details)
    } else {
        map.line_len(begin_folded_line.row())
    };

    (
        map.clip_point(
            DisplayPoint::new(begin_folded_line.row(), new_col),
            Bias::Left,
        ),
        goal,
    )
}

fn down_display(
    map: &DisplaySnapshot,
    mut point: DisplayPoint,
    mut goal: SelectionGoal,
    times: usize,
    text_layout_details: &TextLayoutDetails,
) -> (DisplayPoint, SelectionGoal) {
    for _ in 0..times {
        (point, goal) = movement::down(map, point, goal, true, text_layout_details);
    }

    (point, goal)
}

fn up_display(
    map: &DisplaySnapshot,
    mut point: DisplayPoint,
    mut goal: SelectionGoal,
    times: usize,
    text_layout_details: &TextLayoutDetails,
) -> (DisplayPoint, SelectionGoal) {
    for _ in 0..times {
        (point, goal) = movement::up(map, point, goal, true, text_layout_details);
    }

    (point, goal)
}

pub(crate) fn right(map: &DisplaySnapshot, mut point: DisplayPoint, times: usize) -> DisplayPoint {
    for _ in 0..times {
        let new_point = movement::saturating_right(map, point);
        if point == new_point {
            break;
        }
        point = new_point;
    }
    point
}

pub(crate) fn next_char(
    map: &DisplaySnapshot,
    point: DisplayPoint,
    allow_cross_newline: bool,
) -> DisplayPoint {
    let mut new_point = point;
    let mut max_column = map.line_len(new_point.row());
    if !allow_cross_newline {
        max_column -= 1;
    }
    if new_point.column() < max_column {
        *new_point.column_mut() += 1;
    } else if new_point < map.max_point() && allow_cross_newline {
        *new_point.row_mut() += 1;
        *new_point.column_mut() = 0;
    }
    map.clip_ignoring_line_ends(new_point, Bias::Right)
}

pub(crate) fn next_word_start(
    map: &DisplaySnapshot,
    mut point: DisplayPoint,
    ignore_punctuation: bool,
    times: usize,
) -> DisplayPoint {
    let classifier = map
        .buffer_snapshot
        .char_classifier_at(point.to_point(map))
        .ignore_punctuation(ignore_punctuation);
    for _ in 0..times {
        let mut crossed_newline = false;
        let new_point = movement::find_boundary(map, point, FindRange::MultiLine, |left, right| {
            let left_kind = classifier.kind(left);
            let right_kind = classifier.kind(right);
            let at_newline = right == '\n';

            let found = (left_kind != right_kind && right_kind != CharKind::Whitespace)
                || at_newline && crossed_newline
                || at_newline && left == '\n'; // Prevents skipping repeated empty lines

            crossed_newline |= at_newline;
            found
        });
        if point == new_point {
            break;
        }
        point = new_point;
    }
    point
}

pub(crate) fn next_word_end(
    map: &DisplaySnapshot,
    mut point: DisplayPoint,
    ignore_punctuation: bool,
    times: usize,
    allow_cross_newline: bool,
) -> DisplayPoint {
    let classifier = map
        .buffer_snapshot
        .char_classifier_at(point.to_point(map))
        .ignore_punctuation(ignore_punctuation);
    for _ in 0..times {
        let new_point = next_char(map, point, allow_cross_newline);
        let mut need_next_char = false;
        let new_point = movement::find_boundary_exclusive(
            map,
            new_point,
            FindRange::MultiLine,
            |left, right| {
                let left_kind = classifier.kind(left);
                let right_kind = classifier.kind(right);
                let at_newline = right == '\n';

                if !allow_cross_newline && at_newline {
                    need_next_char = true;
                    return true;
                }

                left_kind != right_kind && left_kind != CharKind::Whitespace
            },
        );
        let new_point = if need_next_char {
            next_char(map, new_point, true)
        } else {
            new_point
        };
        let new_point = map.clip_point(new_point, Bias::Left);
        if point == new_point {
            break;
        }
        point = new_point;
    }
    point
}

fn previous_word_start(
    map: &DisplaySnapshot,
    mut point: DisplayPoint,
    ignore_punctuation: bool,
    times: usize,
) -> DisplayPoint {
    let classifier = map
        .buffer_snapshot
        .char_classifier_at(point.to_point(map))
        .ignore_punctuation(ignore_punctuation);
    for _ in 0..times {
        // This works even though find_preceding_boundary is called for every character in the line containing
        // cursor because the newline is checked only once.
        let new_point = movement::find_preceding_boundary_display_point(
            map,
            point,
            FindRange::MultiLine,
            |left, right| {
                let left_kind = classifier.kind(left);
                let right_kind = classifier.kind(right);

                (left_kind != right_kind && !right.is_whitespace()) || left == '\n'
            },
        );
        if point == new_point {
            break;
        }
        point = new_point;
    }
    point
}

fn previous_word_end(
    map: &DisplaySnapshot,
    point: DisplayPoint,
    ignore_punctuation: bool,
    times: usize,
) -> DisplayPoint {
    let classifier = map
        .buffer_snapshot
        .char_classifier_at(point.to_point(map))
        .ignore_punctuation(ignore_punctuation);
    let mut point = point.to_point(map);

    if point.column < map.buffer_snapshot.line_len(MultiBufferRow(point.row)) {
        point.column += 1;
    }
    for _ in 0..times {
        let new_point = movement::find_preceding_boundary_point(
            &map.buffer_snapshot,
            point,
            FindRange::MultiLine,
            |left, right| {
                let left_kind = classifier.kind(left);
                let right_kind = classifier.kind(right);
                match (left_kind, right_kind) {
                    (CharKind::Punctuation, CharKind::Whitespace)
                    | (CharKind::Punctuation, CharKind::Word)
                    | (CharKind::Word, CharKind::Whitespace)
                    | (CharKind::Word, CharKind::Punctuation) => true,
                    (CharKind::Whitespace, CharKind::Whitespace) => left == '\n' && right == '\n',
                    _ => false,
                }
            },
        );
        if new_point == point {
            break;
        }
        point = new_point;
    }
    movement::saturating_left(map, point.to_display_point(map))
}

fn next_subword_start(
    map: &DisplaySnapshot,
    mut point: DisplayPoint,
    ignore_punctuation: bool,
    times: usize,
) -> DisplayPoint {
    let classifier = map
        .buffer_snapshot
        .char_classifier_at(point.to_point(map))
        .ignore_punctuation(ignore_punctuation);
    for _ in 0..times {
        let mut crossed_newline = false;
        let new_point = movement::find_boundary(map, point, FindRange::MultiLine, |left, right| {
            let left_kind = classifier.kind(left);
            let right_kind = classifier.kind(right);
            let at_newline = right == '\n';

            let is_word_start = (left_kind != right_kind) && !left.is_alphanumeric();
            let is_subword_start =
                left == '_' && right != '_' || left.is_lowercase() && right.is_uppercase();

            let found = (!right.is_whitespace() && (is_word_start || is_subword_start))
                || at_newline && crossed_newline
                || at_newline && left == '\n'; // Prevents skipping repeated empty lines

            crossed_newline |= at_newline;
            found
        });
        if point == new_point {
            break;
        }
        point = new_point;
    }
    point
}

pub(crate) fn next_subword_end(
    map: &DisplaySnapshot,
    mut point: DisplayPoint,
    ignore_punctuation: bool,
    times: usize,
    allow_cross_newline: bool,
) -> DisplayPoint {
    let classifier = map
        .buffer_snapshot
        .char_classifier_at(point.to_point(map))
        .ignore_punctuation(ignore_punctuation);
    for _ in 0..times {
        let new_point = next_char(map, point, allow_cross_newline);

        let mut crossed_newline = false;
        let mut need_backtrack = false;
        let new_point =
            movement::find_boundary(map, new_point, FindRange::MultiLine, |left, right| {
                let left_kind = classifier.kind(left);
                let right_kind = classifier.kind(right);
                let at_newline = right == '\n';

                if !allow_cross_newline && at_newline {
                    return true;
                }

                let is_word_end = (left_kind != right_kind) && !right.is_alphanumeric();
                let is_subword_end =
                    left != '_' && right == '_' || left.is_lowercase() && right.is_uppercase();

                let found = !left.is_whitespace() && !at_newline && (is_word_end || is_subword_end);

                if found && (is_word_end || is_subword_end) {
                    need_backtrack = true;
                }

                crossed_newline |= at_newline;
                found
            });
        let mut new_point = map.clip_point(new_point, Bias::Left);
        if need_backtrack {
            *new_point.column_mut() -= 1;
        }
        if point == new_point {
            break;
        }
        point = new_point;
    }
    point
}

fn previous_subword_start(
    map: &DisplaySnapshot,
    mut point: DisplayPoint,
    ignore_punctuation: bool,
    times: usize,
) -> DisplayPoint {
    let classifier = map
        .buffer_snapshot
        .char_classifier_at(point.to_point(map))
        .ignore_punctuation(ignore_punctuation);
    for _ in 0..times {
        let mut crossed_newline = false;
        // This works even though find_preceding_boundary is called for every character in the line containing
        // cursor because the newline is checked only once.
        let new_point = movement::find_preceding_boundary_display_point(
            map,
            point,
            FindRange::MultiLine,
            |left, right| {
                let left_kind = classifier.kind(left);
                let right_kind = classifier.kind(right);
                let at_newline = right == '\n';

                let is_word_start = (left_kind != right_kind) && !left.is_alphanumeric();
                let is_subword_start =
                    left == '_' && right != '_' || left.is_lowercase() && right.is_uppercase();

                let found = (!right.is_whitespace() && (is_word_start || is_subword_start))
                    || at_newline && crossed_newline
                    || at_newline && left == '\n'; // Prevents skipping repeated empty lines

                crossed_newline |= at_newline;

                found
            },
        );
        if point == new_point {
            break;
        }
        point = new_point;
    }
    point
}

fn previous_subword_end(
    map: &DisplaySnapshot,
    point: DisplayPoint,
    ignore_punctuation: bool,
    times: usize,
) -> DisplayPoint {
    let classifier = map
        .buffer_snapshot
        .char_classifier_at(point.to_point(map))
        .ignore_punctuation(ignore_punctuation);
    let mut point = point.to_point(map);

    if point.column < map.buffer_snapshot.line_len(MultiBufferRow(point.row)) {
        point.column += 1;
    }
    for _ in 0..times {
        let new_point = movement::find_preceding_boundary_point(
            &map.buffer_snapshot,
            point,
            FindRange::MultiLine,
            |left, right| {
                let left_kind = classifier.kind(left);
                let right_kind = classifier.kind(right);

                let is_subword_end =
                    left != '_' && right == '_' || left.is_lowercase() && right.is_uppercase();

                if is_subword_end {
                    return true;
                }

                match (left_kind, right_kind) {
                    (CharKind::Word, CharKind::Whitespace)
                    | (CharKind::Word, CharKind::Punctuation) => true,
                    (CharKind::Whitespace, CharKind::Whitespace) => left == '\n' && right == '\n',
                    _ => false,
                }
            },
        );
        if new_point == point {
            break;
        }
        point = new_point;
    }
    movement::saturating_left(map, point.to_display_point(map))
}

pub(crate) fn first_non_whitespace(
    map: &DisplaySnapshot,
    display_lines: bool,
    from: DisplayPoint,
) -> DisplayPoint {
    let mut start_offset = start_of_line(map, display_lines, from).to_offset(map, Bias::Left);
    let classifier = map.buffer_snapshot.char_classifier_at(from.to_point(map));
    for (ch, offset) in map.buffer_chars_at(start_offset) {
        if ch == '\n' {
            return from;
        }

        start_offset = offset;

        if classifier.kind(ch) != CharKind::Whitespace {
            break;
        }
    }

    start_offset.to_display_point(map)
}

pub(crate) fn last_non_whitespace(
    map: &DisplaySnapshot,
    from: DisplayPoint,
    count: usize,
) -> DisplayPoint {
    let mut end_of_line = end_of_line(map, false, from, count).to_offset(map, Bias::Left);
    let classifier = map.buffer_snapshot.char_classifier_at(from.to_point(map));

    // NOTE: depending on clip_at_line_end we may already be one char back from the end.
    if let Some((ch, _)) = map.buffer_chars_at(end_of_line).next() {
        if classifier.kind(ch) != CharKind::Whitespace {
            return end_of_line.to_display_point(map);
        }
    }

    for (ch, offset) in map.reverse_buffer_chars_at(end_of_line) {
        if ch == '\n' {
            break;
        }
        end_of_line = offset;
        if classifier.kind(ch) != CharKind::Whitespace || ch == '\n' {
            break;
        }
    }

    end_of_line.to_display_point(map)
}

pub(crate) fn start_of_line(
    map: &DisplaySnapshot,
    display_lines: bool,
    point: DisplayPoint,
) -> DisplayPoint {
    if display_lines {
        map.clip_point(DisplayPoint::new(point.row(), 0), Bias::Right)
    } else {
        map.prev_line_boundary(point.to_point(map)).1
    }
}

pub(crate) fn end_of_line(
    map: &DisplaySnapshot,
    display_lines: bool,
    mut point: DisplayPoint,
    times: usize,
) -> DisplayPoint {
    if times > 1 {
        point = start_of_relative_buffer_row(map, point, times as isize - 1);
    }
    if display_lines {
        map.clip_point(
            DisplayPoint::new(point.row(), map.line_len(point.row())),
            Bias::Left,
        )
    } else {
        map.clip_point(map.next_line_boundary(point.to_point(map)).1, Bias::Left)
    }
}

fn sentence_backwards(
    map: &DisplaySnapshot,
    point: DisplayPoint,
    mut times: usize,
) -> DisplayPoint {
    let mut start = point.to_point(map).to_offset(&map.buffer_snapshot);
    let mut chars = map.reverse_buffer_chars_at(start).peekable();

    let mut was_newline = map
        .buffer_chars_at(start)
        .next()
        .is_some_and(|(c, _)| c == '\n');

    while let Some((ch, offset)) = chars.next() {
        let start_of_next_sentence = if was_newline && ch == '\n' {
            Some(offset + ch.len_utf8())
        } else if ch == '\n' && chars.peek().is_some_and(|(c, _)| *c == '\n') {
            Some(next_non_blank(map, offset + ch.len_utf8()))
        } else if ch == '.' || ch == '?' || ch == '!' {
            start_of_next_sentence(map, offset + ch.len_utf8())
        } else {
            None
        };

        if let Some(start_of_next_sentence) = start_of_next_sentence {
            if start_of_next_sentence < start {
                times = times.saturating_sub(1);
            }
            if times == 0 || offset == 0 {
                return map.clip_point(
                    start_of_next_sentence
                        .to_offset(&map.buffer_snapshot)
                        .to_display_point(map),
                    Bias::Left,
                );
            }
        }
        if was_newline {
            start = offset;
        }
        was_newline = ch == '\n';
    }

    DisplayPoint::zero()
}

fn sentence_forwards(map: &DisplaySnapshot, point: DisplayPoint, mut times: usize) -> DisplayPoint {
    let start = point.to_point(map).to_offset(&map.buffer_snapshot);
    let mut chars = map.buffer_chars_at(start).peekable();

    let mut was_newline = map
        .reverse_buffer_chars_at(start)
        .next()
        .is_some_and(|(c, _)| c == '\n')
        && chars.peek().is_some_and(|(c, _)| *c == '\n');

    while let Some((ch, offset)) = chars.next() {
        if was_newline && ch == '\n' {
            continue;
        }
        let start_of_next_sentence = if was_newline {
            Some(next_non_blank(map, offset))
        } else if ch == '\n' && chars.peek().is_some_and(|(c, _)| *c == '\n') {
            Some(next_non_blank(map, offset + ch.len_utf8()))
        } else if ch == '.' || ch == '?' || ch == '!' {
            start_of_next_sentence(map, offset + ch.len_utf8())
        } else {
            None
        };

        if let Some(start_of_next_sentence) = start_of_next_sentence {
            times = times.saturating_sub(1);
            if times == 0 {
                return map.clip_point(
                    start_of_next_sentence
                        .to_offset(&map.buffer_snapshot)
                        .to_display_point(map),
                    Bias::Right,
                );
            }
        }

        was_newline = ch == '\n' && chars.peek().is_some_and(|(c, _)| *c == '\n');
    }

    map.max_point()
}

fn next_non_blank(map: &DisplaySnapshot, start: usize) -> usize {
    for (c, o) in map.buffer_chars_at(start) {
        if c == '\n' || !c.is_whitespace() {
            return o;
        }
    }

    map.buffer_snapshot.len()
}

// given the offset after a ., !, or ? find the start of the next sentence.
// if this is not a sentence boundary, returns None.
fn start_of_next_sentence(map: &DisplaySnapshot, end_of_sentence: usize) -> Option<usize> {
    let chars = map.buffer_chars_at(end_of_sentence);
    let mut seen_space = false;

    for (char, offset) in chars {
        if !seen_space && (char == ')' || char == ']' || char == '"' || char == '\'') {
            continue;
        }

        if char == '\n' && seen_space {
            return Some(offset);
        } else if char.is_whitespace() {
            seen_space = true;
        } else if seen_space {
            return Some(offset);
        } else {
            return None;
        }
    }

    Some(map.buffer_snapshot.len())
}

fn start_of_document(map: &DisplaySnapshot, point: DisplayPoint, line: usize) -> DisplayPoint {
    let mut new_point = Point::new((line - 1) as u32, 0).to_display_point(map);
    *new_point.column_mut() = point.column();
    map.clip_point(new_point, Bias::Left)
}

fn end_of_document(
    map: &DisplaySnapshot,
    point: DisplayPoint,
    line: Option<usize>,
) -> DisplayPoint {
    let new_row = if let Some(line) = line {
        (line - 1) as u32
    } else {
        map.max_buffer_row().0
    };

    let new_point = Point::new(new_row, point.column());
    map.clip_point(new_point.to_display_point(map), Bias::Left)
}

fn matching_tag(map: &DisplaySnapshot, head: DisplayPoint) -> Option<DisplayPoint> {
    let inner = crate::object::surrounding_html_tag(map, head, head..head, false)?;
    let outer = crate::object::surrounding_html_tag(map, head, head..head, true)?;

    if head > outer.start && head < inner.start {
        let mut offset = inner.end.to_offset(map, Bias::Left);
        for c in map.buffer_snapshot.chars_at(offset) {
            if c == '/' || c == '\n' || c == '>' {
                return Some(offset.to_display_point(map));
            }
            offset += c.len_utf8();
        }
    } else {
        let mut offset = outer.start.to_offset(map, Bias::Left);
        for c in map.buffer_snapshot.chars_at(offset) {
            offset += c.len_utf8();
            if c == '<' || c == '\n' {
                return Some(offset.to_display_point(map));
            }
        }
    }

    return None;
}

fn matching(map: &DisplaySnapshot, display_point: DisplayPoint) -> DisplayPoint {
    // https://github.com/vim/vim/blob/1d87e11a1ef201b26ed87585fba70182ad0c468a/runtime/doc/motion.txt#L1200
    let display_point = map.clip_at_line_end(display_point);
    let point = display_point.to_point(map);
    let offset = point.to_offset(&map.buffer_snapshot);

    // Ensure the range is contained by the current line.
    let mut line_end = map.next_line_boundary(point).0;
    if line_end == point {
        line_end = map.max_point().to_point(map);
    }

    let line_range = map.prev_line_boundary(point).0..line_end;
    let visible_line_range =
        line_range.start..Point::new(line_range.end.row, line_range.end.column.saturating_sub(1));
    let ranges = map
        .buffer_snapshot
        .bracket_ranges(visible_line_range.clone());
    if let Some(ranges) = ranges {
        let line_range = line_range.start.to_offset(&map.buffer_snapshot)
            ..line_range.end.to_offset(&map.buffer_snapshot);
        let mut closest_pair_destination = None;
        let mut closest_distance = usize::MAX;

        for (open_range, close_range) in ranges {
            if map.buffer_snapshot.chars_at(open_range.start).next() == Some('<') {
                if offset > open_range.start && offset < close_range.start {
                    let mut chars = map.buffer_snapshot.chars_at(close_range.start);
                    if (Some('/'), Some('>')) == (chars.next(), chars.next()) {
                        return display_point;
                    }
                    if let Some(tag) = matching_tag(map, display_point) {
                        return tag;
                    }
                } else if close_range.contains(&offset) {
                    return open_range.start.to_display_point(map);
                } else if open_range.contains(&offset) {
                    return (close_range.end - 1).to_display_point(map);
                }
            }

            if open_range.start >= offset && line_range.contains(&open_range.start) {
                let distance = open_range.start - offset;
                if distance < closest_distance {
                    closest_pair_destination = Some(close_range.end - 1);
                    closest_distance = distance;
                    continue;
                }
            }

            if close_range.start >= offset && line_range.contains(&close_range.start) {
                let distance = close_range.start - offset;
                if distance < closest_distance {
                    closest_pair_destination = Some(open_range.start);
                    closest_distance = distance;
                    continue;
                }
            }

            continue;
        }

        closest_pair_destination
            .map(|destination| destination.to_display_point(map))
            .unwrap_or(display_point)
    } else {
        display_point
    }
}

fn unmatched_forward(
    map: &DisplaySnapshot,
    mut display_point: DisplayPoint,
    char: char,
    times: usize,
) -> DisplayPoint {
    for _ in 0..times {
        // https://github.com/vim/vim/blob/1d87e11a1ef201b26ed87585fba70182ad0c468a/runtime/doc/motion.txt#L1245
        let point = display_point.to_point(map);
        let offset = point.to_offset(&map.buffer_snapshot);

        let ranges = map.buffer_snapshot.enclosing_bracket_ranges(point..point);
        let Some(ranges) = ranges else { break };
        let mut closest_closing_destination = None;
        let mut closest_distance = usize::MAX;

        for (_, close_range) in ranges {
            if close_range.start > offset {
                let mut chars = map.buffer_snapshot.chars_at(close_range.start);
                if Some(char) == chars.next() {
                    let distance = close_range.start - offset;
                    if distance < closest_distance {
                        closest_closing_destination = Some(close_range.start);
                        closest_distance = distance;
                        continue;
                    }
                }
            }
        }

        let new_point = closest_closing_destination
            .map(|destination| destination.to_display_point(map))
            .unwrap_or(display_point);
        if new_point == display_point {
            break;
        }
        display_point = new_point;
    }
    return display_point;
}

fn unmatched_backward(
    map: &DisplaySnapshot,
    mut display_point: DisplayPoint,
    char: char,
    times: usize,
) -> DisplayPoint {
    for _ in 0..times {
        // https://github.com/vim/vim/blob/1d87e11a1ef201b26ed87585fba70182ad0c468a/runtime/doc/motion.txt#L1239
        let point = display_point.to_point(map);
        let offset = point.to_offset(&map.buffer_snapshot);

        let ranges = map.buffer_snapshot.enclosing_bracket_ranges(point..point);
        let Some(ranges) = ranges else {
            break;
        };

        let mut closest_starting_destination = None;
        let mut closest_distance = usize::MAX;

        for (start_range, _) in ranges {
            if start_range.start < offset {
                let mut chars = map.buffer_snapshot.chars_at(start_range.start);
                if Some(char) == chars.next() {
                    let distance = offset - start_range.start;
                    if distance < closest_distance {
                        closest_starting_destination = Some(start_range.start);
                        closest_distance = distance;
                        continue;
                    }
                }
            }
        }

        let new_point = closest_starting_destination
            .map(|destination| destination.to_display_point(map))
            .unwrap_or(display_point);
        if new_point == display_point {
            break;
        } else {
            display_point = new_point;
        }
    }
    display_point
}

fn find_forward(
    map: &DisplaySnapshot,
    from: DisplayPoint,
    before: bool,
    target: char,
    times: usize,
    mode: FindRange,
    smartcase: bool,
) -> Option<DisplayPoint> {
    let mut to = from;
    let mut found = false;

    for _ in 0..times {
        found = false;
        let new_to = find_boundary(map, to, mode, |_, right| {
            found = is_character_match(target, right, smartcase);
            found
        });
        if to == new_to {
            break;
        }
        to = new_to;
    }

    if found {
        if before && to.column() > 0 {
            *to.column_mut() -= 1;
            Some(map.clip_point(to, Bias::Left))
        } else {
            Some(to)
        }
    } else {
        None
    }
}

fn find_backward(
    map: &DisplaySnapshot,
    from: DisplayPoint,
    after: bool,
    target: char,
    times: usize,
    mode: FindRange,
    smartcase: bool,
) -> DisplayPoint {
    let mut to = from;

    for _ in 0..times {
        let new_to = find_preceding_boundary_display_point(map, to, mode, |_, right| {
            is_character_match(target, right, smartcase)
        });
        if to == new_to {
            break;
        }
        to = new_to;
    }

    let next = map.buffer_snapshot.chars_at(to.to_point(map)).next();
    if next.is_some() && is_character_match(target, next.unwrap(), smartcase) {
        if after {
            *to.column_mut() += 1;
            map.clip_point(to, Bias::Right)
        } else {
            to
        }
    } else {
        from
    }
}

fn is_character_match(target: char, other: char, smartcase: bool) -> bool {
    if smartcase {
        if target.is_uppercase() {
            target == other
        } else {
            target == other.to_ascii_lowercase()
        }
    } else {
        target == other
    }
}

fn next_line_start(map: &DisplaySnapshot, point: DisplayPoint, times: usize) -> DisplayPoint {
    let correct_line = start_of_relative_buffer_row(map, point, times as isize);
    first_non_whitespace(map, false, correct_line)
}

fn previous_line_start(map: &DisplaySnapshot, point: DisplayPoint, times: usize) -> DisplayPoint {
    let correct_line = start_of_relative_buffer_row(map, point, -(times as isize));
    first_non_whitespace(map, false, correct_line)
}

fn go_to_column(map: &DisplaySnapshot, point: DisplayPoint, times: usize) -> DisplayPoint {
    let correct_line = start_of_relative_buffer_row(map, point, 0);
    right(map, correct_line, times.saturating_sub(1))
}

pub(crate) fn next_line_end(
    map: &DisplaySnapshot,
    mut point: DisplayPoint,
    times: usize,
) -> DisplayPoint {
    if times > 1 {
        point = start_of_relative_buffer_row(map, point, times as isize - 1);
    }
    end_of_line(map, false, point, 1)
}

fn window_top(
    map: &DisplaySnapshot,
    point: DisplayPoint,
    text_layout_details: &TextLayoutDetails,
    mut times: usize,
) -> (DisplayPoint, SelectionGoal) {
    let first_visible_line = text_layout_details
        .scroll_anchor
        .anchor
        .to_display_point(map);

    if first_visible_line.row() != DisplayRow(0)
        && text_layout_details.vertical_scroll_margin as usize > times
    {
        times = text_layout_details.vertical_scroll_margin.ceil() as usize;
    }

    if let Some(visible_rows) = text_layout_details.visible_rows {
        let bottom_row = first_visible_line.row().0 + visible_rows as u32;
        let new_row = (first_visible_line.row().0 + (times as u32))
            .min(bottom_row)
            .min(map.max_point().row().0);
        let new_col = point.column().min(map.line_len(first_visible_line.row()));

        let new_point = DisplayPoint::new(DisplayRow(new_row), new_col);
        (map.clip_point(new_point, Bias::Left), SelectionGoal::None)
    } else {
        let new_row =
            DisplayRow((first_visible_line.row().0 + (times as u32)).min(map.max_point().row().0));
        let new_col = point.column().min(map.line_len(first_visible_line.row()));

        let new_point = DisplayPoint::new(new_row, new_col);
        (map.clip_point(new_point, Bias::Left), SelectionGoal::None)
    }
}

fn window_middle(
    map: &DisplaySnapshot,
    point: DisplayPoint,
    text_layout_details: &TextLayoutDetails,
) -> (DisplayPoint, SelectionGoal) {
    if let Some(visible_rows) = text_layout_details.visible_rows {
        let first_visible_line = text_layout_details
            .scroll_anchor
            .anchor
            .to_display_point(map);

        let max_visible_rows =
            (visible_rows as u32).min(map.max_point().row().0 - first_visible_line.row().0);

        let new_row =
            (first_visible_line.row().0 + (max_visible_rows / 2)).min(map.max_point().row().0);
        let new_row = DisplayRow(new_row);
        let new_col = point.column().min(map.line_len(new_row));
        let new_point = DisplayPoint::new(new_row, new_col);
        (map.clip_point(new_point, Bias::Left), SelectionGoal::None)
    } else {
        (point, SelectionGoal::None)
    }
}

fn window_bottom(
    map: &DisplaySnapshot,
    point: DisplayPoint,
    text_layout_details: &TextLayoutDetails,
    mut times: usize,
) -> (DisplayPoint, SelectionGoal) {
    if let Some(visible_rows) = text_layout_details.visible_rows {
        let first_visible_line = text_layout_details
            .scroll_anchor
            .anchor
            .to_display_point(map);
        let bottom_row = first_visible_line.row().0
            + (visible_rows + text_layout_details.scroll_anchor.offset.y - 1.).floor() as u32;
        if bottom_row < map.max_point().row().0
            && text_layout_details.vertical_scroll_margin as usize > times
        {
            times = text_layout_details.vertical_scroll_margin.ceil() as usize;
        }
        let bottom_row_capped = bottom_row.min(map.max_point().row().0);
        let new_row = if bottom_row_capped.saturating_sub(times as u32) < first_visible_line.row().0
        {
            first_visible_line.row()
        } else {
            DisplayRow(bottom_row_capped.saturating_sub(times as u32))
        };
        let new_col = point.column().min(map.line_len(new_row));
        let new_point = DisplayPoint::new(new_row, new_col);
        (map.clip_point(new_point, Bias::Left), SelectionGoal::None)
    } else {
        (point, SelectionGoal::None)
    }
}

#[cfg(test)]
mod test {

    use crate::test::NeovimBackedTestContext;
    use indoc::indoc;

    #[gpui::test]
    async fn test_start_end_of_paragraph(cx: &mut gpui::TestAppContext) {
        let mut cx = NeovimBackedTestContext::new(cx).await;

        let initial_state = indoc! {r"ˇabc
            def

            paragraph
            the second



            third and
            final"};

        // goes down once
        cx.set_shared_state(initial_state).await;
        cx.simulate_shared_keystrokes("}").await;
        cx.shared_state().await.assert_eq(indoc! {r"abc
            def
            ˇ
            paragraph
            the second



            third and
            final"});

        // goes up once
        cx.simulate_shared_keystrokes("{").await;
        cx.shared_state().await.assert_eq(initial_state);

        // goes down twice
        cx.simulate_shared_keystrokes("2 }").await;
        cx.shared_state().await.assert_eq(indoc! {r"abc
            def

            paragraph
            the second
            ˇ


            third and
            final"});

        // goes down over multiple blanks
        cx.simulate_shared_keystrokes("}").await;
        cx.shared_state().await.assert_eq(indoc! {r"abc
                def

                paragraph
                the second



                third and
                finaˇl"});

        // goes up twice
        cx.simulate_shared_keystrokes("2 {").await;
        cx.shared_state().await.assert_eq(indoc! {r"abc
                def
                ˇ
                paragraph
                the second



                third and
                final"});
    }

    #[gpui::test]
    async fn test_matching(cx: &mut gpui::TestAppContext) {
        let mut cx = NeovimBackedTestContext::new(cx).await;

        cx.set_shared_state(indoc! {r"func ˇ(a string) {
                do(something(with<Types>.and_arrays[0, 2]))
            }"})
            .await;
        cx.simulate_shared_keystrokes("%").await;
        cx.shared_state()
            .await
            .assert_eq(indoc! {r"func (a stringˇ) {
                do(something(with<Types>.and_arrays[0, 2]))
            }"});

        // test it works on the last character of the line
        cx.set_shared_state(indoc! {r"func (a string) ˇ{
            do(something(with<Types>.and_arrays[0, 2]))
            }"})
            .await;
        cx.simulate_shared_keystrokes("%").await;
        cx.shared_state()
            .await
            .assert_eq(indoc! {r"func (a string) {
            do(something(with<Types>.and_arrays[0, 2]))
            ˇ}"});

        // test it works on immediate nesting
        cx.set_shared_state("ˇ{()}").await;
        cx.simulate_shared_keystrokes("%").await;
        cx.shared_state().await.assert_eq("{()ˇ}");
        cx.simulate_shared_keystrokes("%").await;
        cx.shared_state().await.assert_eq("ˇ{()}");

        // test it works on immediate nesting inside braces
        cx.set_shared_state("{\n    ˇ{()}\n}").await;
        cx.simulate_shared_keystrokes("%").await;
        cx.shared_state().await.assert_eq("{\n    {()ˇ}\n}");

        // test it jumps to the next paren on a line
        cx.set_shared_state("func ˇboop() {\n}").await;
        cx.simulate_shared_keystrokes("%").await;
        cx.shared_state().await.assert_eq("func boop(ˇ) {\n}");
    }

    #[gpui::test]
    async fn test_unmatched_forward(cx: &mut gpui::TestAppContext) {
        let mut cx = NeovimBackedTestContext::new(cx).await;

        // test it works with curly braces
        cx.set_shared_state(indoc! {r"func (a string) {
                do(something(with<Types>.anˇd_arrays[0, 2]))
            }"})
            .await;
        cx.simulate_shared_keystrokes("] }").await;
        cx.shared_state()
            .await
            .assert_eq(indoc! {r"func (a string) {
                do(something(with<Types>.and_arrays[0, 2]))
            ˇ}"});

        // test it works with brackets
        cx.set_shared_state(indoc! {r"func (a string) {
                do(somethiˇng(with<Types>.and_arrays[0, 2]))
            }"})
            .await;
        cx.simulate_shared_keystrokes("] )").await;
        cx.shared_state()
            .await
            .assert_eq(indoc! {r"func (a string) {
                do(something(with<Types>.and_arrays[0, 2])ˇ)
            }"});

        cx.set_shared_state(indoc! {r"func (a string) { a((b, cˇ))}"})
            .await;
        cx.simulate_shared_keystrokes("] )").await;
        cx.shared_state()
            .await
            .assert_eq(indoc! {r"func (a string) { a((b, c)ˇ)}"});

        // test it works on immediate nesting
        cx.set_shared_state("{ˇ {}{}}").await;
        cx.simulate_shared_keystrokes("] }").await;
        cx.shared_state().await.assert_eq("{ {}{}ˇ}");
        cx.set_shared_state("(ˇ ()())").await;
        cx.simulate_shared_keystrokes("] )").await;
        cx.shared_state().await.assert_eq("( ()()ˇ)");

        // test it works on immediate nesting inside braces
        cx.set_shared_state("{\n    ˇ {()}\n}").await;
        cx.simulate_shared_keystrokes("] }").await;
        cx.shared_state().await.assert_eq("{\n     {()}\nˇ}");
        cx.set_shared_state("(\n    ˇ {()}\n)").await;
        cx.simulate_shared_keystrokes("] )").await;
        cx.shared_state().await.assert_eq("(\n     {()}\nˇ)");
    }

    #[gpui::test]
    async fn test_unmatched_backward(cx: &mut gpui::TestAppContext) {
        let mut cx = NeovimBackedTestContext::new(cx).await;

        // test it works with curly braces
        cx.set_shared_state(indoc! {r"func (a string) {
                do(something(with<Types>.anˇd_arrays[0, 2]))
            }"})
            .await;
        cx.simulate_shared_keystrokes("[ {").await;
        cx.shared_state()
            .await
            .assert_eq(indoc! {r"func (a string) ˇ{
                do(something(with<Types>.and_arrays[0, 2]))
            }"});

        // test it works with brackets
        cx.set_shared_state(indoc! {r"func (a string) {
                do(somethiˇng(with<Types>.and_arrays[0, 2]))
            }"})
            .await;
        cx.simulate_shared_keystrokes("[ (").await;
        cx.shared_state()
            .await
            .assert_eq(indoc! {r"func (a string) {
                doˇ(something(with<Types>.and_arrays[0, 2]))
            }"});

        // test it works on immediate nesting
        cx.set_shared_state("{{}{} ˇ }").await;
        cx.simulate_shared_keystrokes("[ {").await;
        cx.shared_state().await.assert_eq("ˇ{{}{}  }");
        cx.set_shared_state("(()() ˇ )").await;
        cx.simulate_shared_keystrokes("[ (").await;
        cx.shared_state().await.assert_eq("ˇ(()()  )");

        // test it works on immediate nesting inside braces
        cx.set_shared_state("{\n    {()} ˇ\n}").await;
        cx.simulate_shared_keystrokes("[ {").await;
        cx.shared_state().await.assert_eq("ˇ{\n    {()} \n}");
        cx.set_shared_state("(\n    {()} ˇ\n)").await;
        cx.simulate_shared_keystrokes("[ (").await;
        cx.shared_state().await.assert_eq("ˇ(\n    {()} \n)");
    }

    #[gpui::test]
    async fn test_matching_tags(cx: &mut gpui::TestAppContext) {
        let mut cx = NeovimBackedTestContext::new_html(cx).await;

        cx.neovim.exec("set filetype=html").await;

        cx.set_shared_state(indoc! {r"<bˇody></body>"}).await;
        cx.simulate_shared_keystrokes("%").await;
        cx.shared_state()
            .await
            .assert_eq(indoc! {r"<body><ˇ/body>"});
        cx.simulate_shared_keystrokes("%").await;

        // test jumping backwards
        cx.shared_state()
            .await
            .assert_eq(indoc! {r"<ˇbody></body>"});

        // test self-closing tags
        cx.set_shared_state(indoc! {r"<a><bˇr/></a>"}).await;
        cx.simulate_shared_keystrokes("%").await;
        cx.shared_state().await.assert_eq(indoc! {r"<a><bˇr/></a>"});

        // test tag with attributes
        cx.set_shared_state(indoc! {r"<div class='test' ˇid='main'>
            </div>
            "})
            .await;
        cx.simulate_shared_keystrokes("%").await;
        cx.shared_state()
            .await
            .assert_eq(indoc! {r"<div class='test' id='main'>
            <ˇ/div>
            "});

        // test multi-line self-closing tag
        cx.set_shared_state(indoc! {r#"<a>
            <br
                test = "test"
            /ˇ>
        </a>"#})
            .await;
        cx.simulate_shared_keystrokes("%").await;
        cx.shared_state().await.assert_eq(indoc! {r#"<a>
            ˇ<br
                test = "test"
            />
        </a>"#});
    }

    #[gpui::test]
    async fn test_comma_semicolon(cx: &mut gpui::TestAppContext) {
        let mut cx = NeovimBackedTestContext::new(cx).await;

        // f and F
        cx.set_shared_state("ˇone two three four").await;
        cx.simulate_shared_keystrokes("f o").await;
        cx.shared_state().await.assert_eq("one twˇo three four");
        cx.simulate_shared_keystrokes(",").await;
        cx.shared_state().await.assert_eq("ˇone two three four");
        cx.simulate_shared_keystrokes("2 ;").await;
        cx.shared_state().await.assert_eq("one two three fˇour");
        cx.simulate_shared_keystrokes("shift-f e").await;
        cx.shared_state().await.assert_eq("one two threˇe four");
        cx.simulate_shared_keystrokes("2 ;").await;
        cx.shared_state().await.assert_eq("onˇe two three four");
        cx.simulate_shared_keystrokes(",").await;
        cx.shared_state().await.assert_eq("one two thrˇee four");

        // t and T
        cx.set_shared_state("ˇone two three four").await;
        cx.simulate_shared_keystrokes("t o").await;
        cx.shared_state().await.assert_eq("one tˇwo three four");
        cx.simulate_shared_keystrokes(",").await;
        cx.shared_state().await.assert_eq("oˇne two three four");
        cx.simulate_shared_keystrokes("2 ;").await;
        cx.shared_state().await.assert_eq("one two three ˇfour");
        cx.simulate_shared_keystrokes("shift-t e").await;
        cx.shared_state().await.assert_eq("one two threeˇ four");
        cx.simulate_shared_keystrokes("3 ;").await;
        cx.shared_state().await.assert_eq("oneˇ two three four");
        cx.simulate_shared_keystrokes(",").await;
        cx.shared_state().await.assert_eq("one two thˇree four");
    }

    #[gpui::test]
    async fn test_next_word_end_newline_last_char(cx: &mut gpui::TestAppContext) {
        let mut cx = NeovimBackedTestContext::new(cx).await;
        let initial_state = indoc! {r"something(ˇfoo)"};
        cx.set_shared_state(initial_state).await;
        cx.simulate_shared_keystrokes("}").await;
        cx.shared_state().await.assert_eq("something(fooˇ)");
    }

    #[gpui::test]
    async fn test_next_line_start(cx: &mut gpui::TestAppContext) {
        let mut cx = NeovimBackedTestContext::new(cx).await;
        cx.set_shared_state("ˇone\n  two\nthree").await;
        cx.simulate_shared_keystrokes("enter").await;
        cx.shared_state().await.assert_eq("one\n  ˇtwo\nthree");
    }

    #[gpui::test]
    async fn test_end_of_line_downward(cx: &mut gpui::TestAppContext) {
        let mut cx = NeovimBackedTestContext::new(cx).await;
        cx.set_shared_state("ˇ one\n two \nthree").await;
        cx.simulate_shared_keystrokes("g _").await;
        cx.shared_state().await.assert_eq(" onˇe\n two \nthree");

        cx.set_shared_state("ˇ one \n two \nthree").await;
        cx.simulate_shared_keystrokes("g _").await;
        cx.shared_state().await.assert_eq(" onˇe \n two \nthree");
        cx.simulate_shared_keystrokes("2 g _").await;
        cx.shared_state().await.assert_eq(" one \n twˇo \nthree");
    }

    #[gpui::test]
    async fn test_window_top(cx: &mut gpui::TestAppContext) {
        let mut cx = NeovimBackedTestContext::new(cx).await;
        let initial_state = indoc! {r"abc
          def
          paragraph
          the second
          third ˇand
          final"};

        cx.set_shared_state(initial_state).await;
        cx.simulate_shared_keystrokes("shift-h").await;
        cx.shared_state().await.assert_eq(indoc! {r"abˇc
          def
          paragraph
          the second
          third and
          final"});

        // clip point
        cx.set_shared_state(indoc! {r"
          1 2 3
          4 5 6
          7 8 ˇ9
          "})
            .await;
        cx.simulate_shared_keystrokes("shift-h").await;
        cx.shared_state().await.assert_eq(indoc! {"
          1 2 ˇ3
          4 5 6
          7 8 9
          "});

        cx.set_shared_state(indoc! {r"
          1 2 3
          4 5 6
          ˇ7 8 9
          "})
            .await;
        cx.simulate_shared_keystrokes("shift-h").await;
        cx.shared_state().await.assert_eq(indoc! {"
          ˇ1 2 3
          4 5 6
          7 8 9
          "});

        cx.set_shared_state(indoc! {r"
          1 2 3
          4 5 ˇ6
          7 8 9"})
            .await;
        cx.simulate_shared_keystrokes("9 shift-h").await;
        cx.shared_state().await.assert_eq(indoc! {"
          1 2 3
          4 5 6
          7 8 ˇ9"});
    }

    #[gpui::test]
    async fn test_window_middle(cx: &mut gpui::TestAppContext) {
        let mut cx = NeovimBackedTestContext::new(cx).await;
        let initial_state = indoc! {r"abˇc
          def
          paragraph
          the second
          third and
          final"};

        cx.set_shared_state(initial_state).await;
        cx.simulate_shared_keystrokes("shift-m").await;
        cx.shared_state().await.assert_eq(indoc! {r"abc
          def
          paˇragraph
          the second
          third and
          final"});

        cx.set_shared_state(indoc! {r"
          1 2 3
          4 5 6
          7 8 ˇ9
          "})
            .await;
        cx.simulate_shared_keystrokes("shift-m").await;
        cx.shared_state().await.assert_eq(indoc! {"
          1 2 3
          4 5 ˇ6
          7 8 9
          "});
        cx.set_shared_state(indoc! {r"
          1 2 3
          4 5 6
          ˇ7 8 9
          "})
            .await;
        cx.simulate_shared_keystrokes("shift-m").await;
        cx.shared_state().await.assert_eq(indoc! {"
          1 2 3
          ˇ4 5 6
          7 8 9
          "});
        cx.set_shared_state(indoc! {r"
          ˇ1 2 3
          4 5 6
          7 8 9
          "})
            .await;
        cx.simulate_shared_keystrokes("shift-m").await;
        cx.shared_state().await.assert_eq(indoc! {"
          1 2 3
          ˇ4 5 6
          7 8 9
          "});
        cx.set_shared_state(indoc! {r"
          1 2 3
          ˇ4 5 6
          7 8 9
          "})
            .await;
        cx.simulate_shared_keystrokes("shift-m").await;
        cx.shared_state().await.assert_eq(indoc! {"
          1 2 3
          ˇ4 5 6
          7 8 9
          "});
        cx.set_shared_state(indoc! {r"
          1 2 3
          4 5 ˇ6
          7 8 9
          "})
            .await;
        cx.simulate_shared_keystrokes("shift-m").await;
        cx.shared_state().await.assert_eq(indoc! {"
          1 2 3
          4 5 ˇ6
          7 8 9
          "});
    }

    #[gpui::test]
    async fn test_window_bottom(cx: &mut gpui::TestAppContext) {
        let mut cx = NeovimBackedTestContext::new(cx).await;
        let initial_state = indoc! {r"abc
          deˇf
          paragraph
          the second
          third and
          final"};

        cx.set_shared_state(initial_state).await;
        cx.simulate_shared_keystrokes("shift-l").await;
        cx.shared_state().await.assert_eq(indoc! {r"abc
          def
          paragraph
          the second
          third and
          fiˇnal"});

        cx.set_shared_state(indoc! {r"
          1 2 3
          4 5 ˇ6
          7 8 9
          "})
            .await;
        cx.simulate_shared_keystrokes("shift-l").await;
        cx.shared_state().await.assert_eq(indoc! {"
          1 2 3
          4 5 6
          7 8 9
          ˇ"});

        cx.set_shared_state(indoc! {r"
          1 2 3
          ˇ4 5 6
          7 8 9
          "})
            .await;
        cx.simulate_shared_keystrokes("shift-l").await;
        cx.shared_state().await.assert_eq(indoc! {"
          1 2 3
          4 5 6
          7 8 9
          ˇ"});

        cx.set_shared_state(indoc! {r"
          1 2 ˇ3
          4 5 6
          7 8 9
          "})
            .await;
        cx.simulate_shared_keystrokes("shift-l").await;
        cx.shared_state().await.assert_eq(indoc! {"
          1 2 3
          4 5 6
          7 8 9
          ˇ"});

        cx.set_shared_state(indoc! {r"
          ˇ1 2 3
          4 5 6
          7 8 9
          "})
            .await;
        cx.simulate_shared_keystrokes("shift-l").await;
        cx.shared_state().await.assert_eq(indoc! {"
          1 2 3
          4 5 6
          7 8 9
          ˇ"});

        cx.set_shared_state(indoc! {r"
          1 2 3
          4 5 ˇ6
          7 8 9
          "})
            .await;
        cx.simulate_shared_keystrokes("9 shift-l").await;
        cx.shared_state().await.assert_eq(indoc! {"
          1 2 ˇ3
          4 5 6
          7 8 9
          "});
    }

    #[gpui::test]
    async fn test_previous_word_end(cx: &mut gpui::TestAppContext) {
        let mut cx = NeovimBackedTestContext::new(cx).await;
        cx.set_shared_state(indoc! {r"
        456 5ˇ67 678
        "})
            .await;
        cx.simulate_shared_keystrokes("g e").await;
        cx.shared_state().await.assert_eq(indoc! {"
        45ˇ6 567 678
        "});

        // Test times
        cx.set_shared_state(indoc! {r"
        123 234 345
        456 5ˇ67 678
        "})
            .await;
        cx.simulate_shared_keystrokes("4 g e").await;
        cx.shared_state().await.assert_eq(indoc! {"
        12ˇ3 234 345
        456 567 678
        "});

        // With punctuation
        cx.set_shared_state(indoc! {r"
        123 234 345
        4;5.6 5ˇ67 678
        789 890 901
        "})
            .await;
        cx.simulate_shared_keystrokes("g e").await;
        cx.shared_state().await.assert_eq(indoc! {"
          123 234 345
          4;5.ˇ6 567 678
          789 890 901
        "});

        // With punctuation and count
        cx.set_shared_state(indoc! {r"
        123 234 345
        4;5.6 5ˇ67 678
        789 890 901
        "})
            .await;
        cx.simulate_shared_keystrokes("5 g e").await;
        cx.shared_state().await.assert_eq(indoc! {"
          123 234 345
          ˇ4;5.6 567 678
          789 890 901
        "});

        // newlines
        cx.set_shared_state(indoc! {r"
        123 234 345

        78ˇ9 890 901
        "})
            .await;
        cx.simulate_shared_keystrokes("g e").await;
        cx.shared_state().await.assert_eq(indoc! {"
          123 234 345
          ˇ
          789 890 901
        "});
        cx.simulate_shared_keystrokes("g e").await;
        cx.shared_state().await.assert_eq(indoc! {"
          123 234 34ˇ5

          789 890 901
        "});

        // With punctuation
        cx.set_shared_state(indoc! {r"
        123 234 345
        4;5.ˇ6 567 678
        789 890 901
        "})
            .await;
        cx.simulate_shared_keystrokes("g shift-e").await;
        cx.shared_state().await.assert_eq(indoc! {"
          123 234 34ˇ5
          4;5.6 567 678
          789 890 901
        "});
    }

    #[gpui::test]
    async fn test_visual_match_eol(cx: &mut gpui::TestAppContext) {
        let mut cx = NeovimBackedTestContext::new(cx).await;

        cx.set_shared_state(indoc! {"
            fn aˇ() {
              return
            }
        "})
            .await;
        cx.simulate_shared_keystrokes("v $ %").await;
        cx.shared_state().await.assert_eq(indoc! {"
            fn a«() {
              return
            }ˇ»
        "});
    }
}
