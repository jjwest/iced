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
    column_max: u8,
    row_max: u8,
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
        row: u8,
        column: u8,
    ) -> Self {
        self.cells(child, row, row, column, column)
    }

    /// Add a multi-cell entry to the grid.
    pub fn cells(
        mut self,
        child: impl Into<Element<'a, Message, Theme, Renderer>>,
        row_start: u8,
        row_end: u8,
        column_start: u8,
        column_end: u8,
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

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, _limits: &Limits) -> Node {
        let mut column_widths = vec![0.0f32; (self.column_max + 1) as usize];

        for column in 0..self.column_max + 1 {
            for ((child, layout), tree) in self
                .children
                .iter_mut()
                .zip(&self.child_layouts)
                .zip(&mut tree.children)
            {
                if layout.column_start == column && layout.column_end == column {
                    column_widths[column as usize] = column_widths[column as usize].max(
                        child
                            .as_widget_mut()
                            .layout(tree, renderer, &Limits::NONE.width(Length::Shrink))
                            .size()
                            .width,
                    )
                }
            }
        }

        let mut row_heights = vec![0.0f32; (self.row_max + 1) as usize];

        for row in 0..self.row_max + 1 {
            for ((child, layout), tree) in self
                .children
                .iter_mut()
                .zip(&self.child_layouts)
                .zip(&mut tree.children)
            {
                if layout.row_start == row && layout.row_end == row {
                    row_heights[row as usize] = row_heights[row as usize].max(
                        child
                            .as_widget_mut()
                            .layout(tree, renderer, &Limits::NONE.height(Length::Shrink))
                            .size()
                            .height,
                    )
                }
            }
        }

        let mut child_nodes = Vec::with_capacity(self.children.len());

        let mut width = 0.0f32;
        let mut height = 0.0f32;

        let mut row_offset = 0.0;
        for (row, &row_height) in row_heights.iter().enumerate() {
            let row = row as u8;
            let mut column_offset = 0.0;
            for (column, &column_width) in column_widths.iter().enumerate() {
                let column = column as u8;
                for ((child, layout), tree) in self
                    .children
                    .iter_mut()
                    .zip(&self.child_layouts)
                    .zip(&mut tree.children)
                {
                    if layout.row_start == row
                        && layout.row_end == row
                        && layout.column_start == column
                        && layout.column_end == column
                    {
                        let limits = Limits::new(Size::ZERO, Size::new(column_width, row_height));
                        let node = child.as_widget_mut().layout(tree, renderer, &limits);
                        child_nodes.push(node.move_to(Point::new(column_offset, row_offset)));
                    }
                }
                column_offset += column_width + self.spacing.width;
                width = width.max(column_offset);
            }
            row_offset += row_height + self.spacing.height;
            height = height.max(row_offset);
        }

        Node::with_children(Size::new(width, height), child_nodes)
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
    column_start: u8,
    column_end: u8,
    row_start: u8,
    row_end: u8,
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
