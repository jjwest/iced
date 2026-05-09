//! Keyed stacks distribute content vertically while keeping continuity.

use crate::core::layout;
use crate::core::mouse;
use crate::core::overlay;
use crate::core::renderer;
use crate::core::widget::Operation;
use crate::core::widget::tree::{self, Tree};
use crate::core::{Element, Event, Layout, Length, Rectangle, Shell, Size, Vector, Widget};

/// A container that distributes its contents vertically while keeping continuity.
///
/// # Example
/// ```no_run
/// # mod iced { pub mod widget { pub use iced_widget::*; } }
/// # pub type State = ();
/// # pub type Element<'a, Message> = iced_widget::core::Element<'a, Message, iced_widget::Theme, iced_widget::Renderer>;
/// use iced::widget::{keyed_stack, text};
///
/// enum Message {
///     // ...
/// }
///
/// fn view(state: &State) -> Element<'_, Message> {
///     keyed_stack((0..=100).map(|i| {
///         (i, text!("Item {i}").into())
///     })).into()
/// }
/// ```
pub struct Stack<'a, Key, Message, Theme = crate::Theme, Renderer = crate::Renderer>
where
    Key: Copy + PartialEq,
{
    width: Length,
    height: Length,
    keys: Vec<Key>,
    children: Vec<Element<'a, Message, Theme, Renderer>>,
    clip: bool,
    base_layer: usize,
}

impl<'a, Key, Message, Theme, Renderer> Stack<'a, Key, Message, Theme, Renderer>
where
    Key: Copy + PartialEq,
    Renderer: crate::core::Renderer,
{
    /// Creates an empty [`Stack`].
    pub fn new() -> Self {
        Self::from_vecs(Vec::new(), Vec::new())
    }

    /// Creates a [`Stack`] from already allocated [`Vec`]s.
    ///
    /// Keep in mind that the [`Stack`] will not inspect the [`Vec`]s, which means
    /// it won't automatically adapt to the sizing strategy of its contents.
    ///
    /// If any of the children have a [`Length::Fill`] strategy, you will need to
    /// call [`Stack::width`] or [`Stack::height`] accordingly.
    pub fn from_vecs(keys: Vec<Key>, children: Vec<Element<'a, Message, Theme, Renderer>>) -> Self {
        Self {
            width: Length::Shrink,
            height: Length::Shrink,
            keys,
            children,
            clip: false,
            base_layer: 0,
        }
    }

    /// Creates a [`Stack`] with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self::from_vecs(Vec::with_capacity(capacity), Vec::with_capacity(capacity))
    }

    /// Creates a [`Stack`] with the given elements.
    pub fn with_children(
        children: impl IntoIterator<Item = (Key, Element<'a, Message, Theme, Renderer>)>,
    ) -> Self {
        let iterator = children.into_iter();

        Self::with_capacity(iterator.size_hint().0).extend(iterator)
    }

    /// Sets the vertical spacing _between_ elements.
    ///
    /// Sets the width of the [`Stack`].
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height of the [`Stack`].
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Adds an element to the [`Stack`].
    pub fn push(
        mut self,
        key: Key,
        child: impl Into<Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        let child = child.into();
        let child_size = child.as_widget().size_hint();

        self.width = self.width.enclose(child_size.width);
        self.height = self.height.enclose(child_size.height);

        self.keys.push(key);
        self.children.push(child);
        self
    }

    /// Adds an element to the [`Stack`], if `Some`.
    pub fn push_maybe(
        self,
        key: Key,
        child: Option<impl Into<Element<'a, Message, Theme, Renderer>>>,
    ) -> Self {
        if let Some(child) = child {
            self.push(key, child)
        } else {
            self
        }
    }

    /// Extends the [`Stack`] with the given children.
    pub fn extend(
        self,
        children: impl IntoIterator<Item = (Key, Element<'a, Message, Theme, Renderer>)>,
    ) -> Self {
        children
            .into_iter()
            .fold(self, |stack, (key, child)| stack.push(key, child))
    }
}

impl<Key, Message, Renderer> Default for Stack<'_, Key, Message, Renderer>
where
    Key: Copy + PartialEq,
    Renderer: crate::core::Renderer,
{
    fn default() -> Self {
        Self::new()
    }
}

struct State<Key>
where
    Key: Copy + PartialEq,
{
    keys: Vec<Key>,
}

impl<'a, Key, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Stack<'a, Key, Message, Theme, Renderer>
where
    Renderer: crate::core::Renderer,
    Key: Copy + PartialEq + 'static,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State<Key>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State {
            keys: self.keys.clone(),
        })
    }

    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        let Tree {
            state, children, ..
        } = tree;

        let state = state.downcast_mut::<State<Key>>();

        let mut to_remove = Vec::new();
        for (old_idx, old_key) in state.keys.iter().enumerate() {
            if !self.keys.contains(old_key) {
                to_remove.push(old_idx);
            }
        }
        let mut removed = 0;
        for idx in to_remove {
            let _ = state.keys.remove(idx - removed);
            let _ = children.remove(idx - removed);
            removed += 1;
        }

        for (new_idx, new_key) in self.keys.iter().enumerate() {
            if !state.keys.contains(new_key) {
                state.keys.push(*new_key);
                children.push(Tree::new(self.children[new_idx].as_widget()));
            }
        }

        let mut swapped = Vec::new();
        for (new_idx, new_key) in self.keys.iter().enumerate() {
            if let Some(current_idx) = state.keys.iter().position(|key| *key == *new_key) {
                if current_idx != new_idx && !swapped.contains(new_key) {
                    swapped.push(new_key.clone());
                    swapped.push(state.keys[new_idx].clone());
                    children.swap(current_idx, new_idx);
                }
                self.children[new_idx]
                    .as_widget()
                    .diff(&mut children[new_idx]);
            } else {
                children.insert(new_idx, Tree::new(self.children[new_idx].as_widget()));
            }
        }

        if state.keys != self.keys {
            state.keys.clone_from(&self.keys);
        }
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let limits = limits.width(self.width).height(self.height);

        if self.children.len() <= self.base_layer {
            return layout::Node::new(limits.resolve(self.width, self.height, Size::ZERO));
        }

        let base = self.children[self.base_layer].as_widget_mut().layout(
            &mut tree.children[self.base_layer],
            renderer,
            &limits,
        );

        let size = limits.resolve(self.width, self.height, base.size());
        let limits = layout::Limits::new(Size::ZERO, size);

        let (under, above) = self.children.split_at_mut(self.base_layer);
        let (tree_under, tree_above) = tree.children.split_at_mut(self.base_layer);

        let nodes = under
            .iter_mut()
            .zip(tree_under)
            .map(|(layer, tree)| layer.as_widget_mut().layout(tree, renderer, &limits))
            .chain(std::iter::once(base))
            .chain(
                above[1..]
                    .iter_mut()
                    .zip(&mut tree_above[1..])
                    .map(|(layer, tree)| layer.as_widget_mut().layout(tree, renderer, &limits)),
            )
            .collect();

        layout::Node::with_children(size, nodes)
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
        mut cursor: mouse::Cursor,
        renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        if self.children.is_empty() {
            return;
        }

        let is_over = cursor.is_over(layout.bounds());
        let end = self.children.len() - 1;

        for (i, ((child, tree), layout)) in self
            .children
            .iter_mut()
            .rev()
            .zip(tree.children.iter_mut().rev())
            .zip(layout.children().rev())
            .enumerate()
        {
            if shell.is_event_captured() {
                child.as_widget_mut().update(
                    tree,
                    &Event::Mouse(mouse::Event::CursorLeft),
                    layout,
                    cursor,
                    renderer,
                    shell,
                    viewport,
                );
                continue;
            }

            child
                .as_widget_mut()
                .update(tree, event, layout, cursor, renderer, shell, viewport);

            if i < end && is_over && !cursor.is_levitating() {
                let interaction = child
                    .as_widget()
                    .mouse_interaction(tree, layout, cursor, viewport, renderer);

                if interaction != mouse::Interaction::None {
                    cursor = cursor.levitate();
                }
            }
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
            .rev()
            .zip(tree.children.iter().rev())
            .zip(layout.children().rev())
            .map(|((child, tree), layout)| {
                child
                    .as_widget()
                    .mouse_interaction(tree, layout, cursor, viewport, renderer)
            })
            .find(|&interaction| interaction != mouse::Interaction::None)
            .unwrap_or_default()
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        if let Some(clipped_viewport) = layout.bounds().intersection(viewport) {
            let viewport = if self.clip {
                &clipped_viewport
            } else {
                viewport
            };

            let layers_under = if cursor.is_over(layout.bounds()) {
                self.children
                    .iter()
                    .rev()
                    .zip(tree.children.iter().rev())
                    .zip(layout.children().rev())
                    .position(|((layer, tree), layout)| {
                        let interaction = layer
                            .as_widget()
                            .mouse_interaction(tree, layout, cursor, viewport, renderer);

                        interaction != mouse::Interaction::None
                    })
                    .map(|i| self.children.len() - i - 1)
                    .unwrap_or_default()
            } else {
                0
            };

            let mut layers = self
                .children
                .iter()
                .zip(&tree.children)
                .zip(layout.children())
                .enumerate();

            let layers = layers.by_ref();

            let mut draw_layer =
                |i, layer: &Element<'a, Message, Theme, Renderer>, tree, layout, cursor| {
                    if i > 0 {
                        renderer.with_layer(*viewport, |renderer| {
                            layer
                                .as_widget()
                                .draw(tree, renderer, theme, style, layout, cursor, viewport);
                        });
                    } else {
                        layer
                            .as_widget()
                            .draw(tree, renderer, theme, style, layout, cursor, viewport);
                    }
                };

            for (i, ((layer, tree), layout)) in layers.take(layers_under) {
                draw_layer(i, layer, tree, layout, mouse::Cursor::Unavailable);
            }

            for (i, ((layer, tree), layout)) in layers {
                draw_layer(i, layer, tree, layout, cursor);
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
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
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

impl<'a, Key, Message, Theme, Renderer> From<Stack<'a, Key, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Key: Copy + PartialEq + 'static,
    Message: 'a,
    Theme: 'a,
    Renderer: crate::core::Renderer + 'a,
{
    fn from(stack: Stack<'a, Key, Message, Theme, Renderer>) -> Self {
        Self::new(stack)
    }
}
