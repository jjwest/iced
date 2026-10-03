//! Distribute content on a grid.

use crate::core::layout::{Limits, Node};
use crate::core::mouse::{self, Cursor};
use crate::core::overlay;
use crate::core::renderer::Style;
use crate::core::widget::{Operation, Tree};
use crate::core::{Element, Event, Layout, Length, Point, Rectangle, Shell, Size, Vector, Widget};

/// A container that distributes its contents on a responsive grid.
pub struct Grid<'a, Message, Theme = crate::Theme, Renderer = crate::Renderer> {
    width: Length,
    height: Length,
    children: Vec<Element<'a, Message, Theme, Renderer>>,
    child_layouts: Vec<ChildLayout>,
    column_max: usize,
    row_max: usize,
    spacing: Size,
}

impl<'a, Message, Theme, Renderer> Grid<'a, Message, Theme, Renderer>
where
    Renderer: crate::core::Renderer,
{
    /// Create a new Grid widget
    pub fn new() -> Self {
        Self {
            width: Length::Fit,
            height: Length::Fit,
            children: Vec::new(),
            child_layouts: Vec::new(),
            column_max: 0,
            row_max: 0,
            spacing: Size::ZERO,
        }
    }

    /// Add a single-cell entry to the grid.
    pub fn cell(
        self,
        child: impl Into<Element<'a, Message, Theme, Renderer>>,
        row: usize,
        column: usize,
    ) -> Self {
        self.cells(child, row, row, column, column)
    }

    /// Add a multi-cell entry to the grid.
    pub fn cells(
        mut self,
        child: impl Into<Element<'a, Message, Theme, Renderer>>,
        row_start: usize,
        row_end: usize,
        column_start: usize,
        column_end: usize,
    ) -> Self {
        assert!(row_start <= row_end);
        assert!(column_start <= column_end);
        self.column_max = self.column_max.max(column_end);
        self.row_max = self.row_max.max(row_end);
        self.children.push(child.into());
        self.child_layouts.push(ChildLayout {
            row_start,
            row_end,
            column_start,
            column_end,
        });
        self
    }

    /// Set the width of the grid.
    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    /// Set the height of the grid.
    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    /// Set the spacing for the grid columns.
    pub fn spacing_x(mut self, amount: f32) -> Self {
        self.spacing.width = amount;
        self
    }

    /// Set the spacing for the grid rows.
    pub fn spacing_y(mut self, amount: f32) -> Self {
        self.spacing.height = amount;
        self
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Grid<'a, Message, Theme, Renderer>
where
    Renderer: crate::core::Renderer,
{
    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
    }

    fn diff(&mut self, tree: &mut Tree) {
        tree.children.resize_with(self.children.len(), Tree::empty);
        for (old_child, new_child) in tree.children.iter_mut().zip(&mut self.children) {
            old_child.diff(new_child);
        }
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        #[derive(Clone, Copy)]
        struct Meta {
            pos: f32,
            size: f32,
        }

        // First pass.
        // Gather the minimum width for each column based on
        // cells that occupy just that column.
        let mut column_widths = vec![
            Meta {
                pos: 0.0,
                size: 0.0
            };
            self.column_max + 1
        ];
        for column in 0..self.column_max + 1 {
            for ((child, tree), layout) in self
                .children
                .iter_mut()
                .zip(&mut tree.children)
                .zip(&self.child_layouts)
            {
                if layout.column_start == column && layout.column_end == column {
                    column_widths[column].size = column_widths[column].size.max(
                        child
                            .as_widget_mut()
                            .layout(tree, renderer, &Limits::NONE.width(Length::Shrink))
                            .size()
                            .width,
                    )
                }
            }
            if column > 0 {
                column_widths[column].pos = column_widths[column - 1].pos
                    + column_widths[column - 1].size
                    + self.spacing.width;
            }
        }

        // Second pass.
        // Gather the minimum height for each row based on
        // cells that occupy just that row.
        let mut row_heights = vec![
            Meta {
                pos: 0.0,
                size: 0.0
            };
            self.row_max + 1
        ];
        for row in 0..self.row_max + 1 {
            for ((child, tree), layout) in self
                .children
                .iter_mut()
                .zip(&mut tree.children)
                .zip(&self.child_layouts)
            {
                if layout.row_start == row && layout.row_end == row {
                    row_heights[row].size = row_heights[row].size.max(
                        child
                            .as_widget_mut()
                            .layout(tree, renderer, &Limits::NONE.height(Length::Shrink))
                            .size()
                            .height,
                    )
                }
            }
            if row > 0 {
                row_heights[row].pos =
                    row_heights[row - 1].pos + row_heights[row - 1].size + self.spacing.height;
            }
        }

        // Third pass.
        // Adjust the width of columns and heights of rows if necessary,
        // taking into account multi-column and multi-row cells, but ignoring Fills.
        for ((child, tree), layout) in self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(&self.child_layouts)
        {
            if layout.column_start != layout.column_end || layout.row_start != layout.row_end {
                let sizing = child.as_widget().size();
                let size = child
                    .as_widget_mut()
                    .layout(
                        tree,
                        renderer,
                        &Limits::NONE.width(Length::Shrink).height(Length::Shrink),
                    )
                    .size();
                if !matches!(sizing.width, Length::Fill | Length::FillPortion(..)) {
                    let total_width = column_widths[layout.column_start..layout.column_end + 1]
                        .iter()
                        .map(|meta| meta.size)
                        .sum();
                    if size.width > total_width && layout.column_end < self.column_max {
                        let diff = size.width - total_width;
                        column_widths[layout.column_end].size += diff;
                        for meta in &mut column_widths[layout.column_end + 1..] {
                            meta.pos += diff;
                        }
                    }
                }
                if !matches!(sizing.height, Length::Fill | Length::FillPortion(..)) {
                    let total_height = row_heights[layout.row_start..layout.row_end + 1]
                        .iter()
                        .map(|meta| meta.size)
                        .sum();
                    if size.height > total_height && layout.row_end < self.row_max {
                        let diff = size.height - total_height;
                        row_heights[layout.row_end].size += diff;
                        for meta in &mut row_heights[layout.row_end + 1..] {
                            meta.pos += diff;
                        }
                    }
                }
            }
        }

        let intrinsic_size = Size::new(
            column_widths[self.column_max].pos + column_widths[self.column_max].size,
            row_heights[self.row_max].pos + row_heights[self.row_max].size,
        );
        let size = limits.resolve(self.width, self.height, intrinsic_size);

        if size.width > intrinsic_size.width {
            let extra_width = size.width - intrinsic_size.width;
            for meta in &mut column_widths {
                let factor = meta.size / intrinsic_size.width;
                meta.size += extra_width * factor;
            }
            for i in 1..column_widths.len() {
                column_widths[i].pos =
                    column_widths[i - 1].pos + column_widths[i - 1].size + self.spacing.width;
            }
        }

        if size.height > intrinsic_size.height {
            let extra_height = size.height - intrinsic_size.height;
            for meta in &mut row_heights {
                let factor = meta.size / intrinsic_size.height;
                meta.size += extra_height * factor;
            }
            for i in 1..row_heights.len() {
                row_heights[i].pos =
                    row_heights[i - 1].pos + row_heights[i - 1].size + self.spacing.height;
            }
        }

        let mut child_nodes = Vec::with_capacity(self.children.len());

        for ((child, tree), layout) in self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(&self.child_layouts)
        {
            let x = column_widths[layout.column_start].pos;
            let y = row_heights[layout.row_start].pos;
            let child_width = column_widths[layout.column_start..layout.column_end + 1]
                .iter()
                .map(|meta| meta.size)
                .sum();
            let child_height = row_heights[layout.row_start..layout.row_end + 1]
                .iter()
                .map(|meta| meta.size)
                .sum();
            let limits = Limits::new(Size::ZERO, Size::new(child_width, child_height));
            let node = child.as_widget_mut().layout(tree, renderer, &limits);
            child_nodes.push(node.move_to(Point::new(x, y)));
        }

        Node::with_children(size, child_nodes)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.children
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
                .for_each(|((child, state), layout)| {
                    child
                        .as_widget_mut()
                        .operate(state, layout, renderer, operation);
                });
        });
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        for ((child, tree), layout) in self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            child
                .as_widget_mut()
                .update(tree, event, layout, cursor, renderer, shell, viewport);
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .map(|((child, tree), layout)| {
                child
                    .as_widget()
                    .mouse_interaction(tree, layout, cursor, viewport, renderer)
            })
            .max()
            .unwrap_or_default()
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &Style,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
    ) {
        if let Some(viewport) = layout.bounds().intersection(viewport) {
            renderer.with_layer(viewport, |renderer| {
                for ((child, tree), layout) in self
                    .children
                    .iter()
                    .zip(&tree.children)
                    .zip(layout.children())
                    .filter(|(_, layout)| layout.bounds().intersects(&viewport))
                {
                    child
                        .as_widget()
                        .draw(tree, renderer, theme, style, layout, cursor, &viewport);
                }
            });
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Vec<overlay::Element<'b, Message, Theme, Renderer>> {
        overlay::from_children(
            &mut self.children,
            tree,
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

struct ChildLayout {
    column_start: usize,
    column_end: usize,
    row_start: usize,
    row_end: usize,
}

impl<'a, Message, Theme, Renderer> From<Grid<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: crate::core::Renderer + 'a,
{
    fn from(row: Grid<'a, Message, Theme, Renderer>) -> Self {
        Self::new(row)
    }
}
