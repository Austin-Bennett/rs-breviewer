
use std::cell::{Ref, RefCell, RefMut};
use std::collections::{HashSet, VecDeque};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::{Stylize, Widget};
use ratatui::style::{Color, Modifier, Style, Styled};
use ratatui::Frame;
use ratatui::widgets::Block;
use ratatui_textarea::TextArea;
use crate::review::Reviews;

// shared highlight color for both the currently-hovered node's border and,
// where applicable, its own internal notion of "hovered" (e.g. Reviews'
// grid cells) - kept in one place so they can't drift apart.
pub const HOVER_COLOR: Color = Color::White;

pub trait DynUINode {

    // returns true if this node should no longer be selected (e.g. a plain
    // text area returns true on Esc; Reviews returns true on Esc only if it
    // doesn't currently have one of its own inner text areas selected)
    fn handle_input(&mut self, key: KeyEvent) -> bool;

    fn select(&mut self);
    fn deselect(&mut self);

    // called when this node becomes the hovered/focused node
    fn hover(&mut self);
    // called when this node stops being the hovered/focused node
    fn unhover(&mut self);

    fn up(&self) -> RawUINode;
    fn right(&self) -> RawUINode;
    fn down(&self) -> RawUINode;
    fn left(&self) -> RawUINode;

    // link setters, callable through the type-erased dyn object so builders
    // don't need to know each node's concrete widget type
    fn set_up(&mut self, node: RawUINode);
    fn set_right(&mut self, node: RawUINode);
    fn set_down(&mut self, node: RawUINode);
    fn set_left(&mut self, node: RawUINode);

    fn set_area(&mut self, area: AreaDescription);

    // draws the widget into its area of the frame, computed from its AreaDescription
    fn draw(&self, frame: &mut Frame);
}

pub type RawUINode = Option<ValidUINode>;
pub type ValidUINode = Rc<RefCell<dyn DynUINode>>;

//describes how a widget takes up the area of the terminal
#[derive(Clone, Copy)]
pub struct AreaDescription {
    // each is a percentage from 0 to 100 describing the percent area of the terminal it uses
    pub x: u8,
    pub y: u8,
    pub w: u8,
    pub h: u8,
}

impl AreaDescription {
    pub fn new(x: u8, y: u8, w: u8, h: u8) -> Self {
        Self{
            x, y, w, h
        }
    }

    fn left(&self) -> i32 { self.x as i32 }
    fn right(&self) -> i32 { self.x as i32 + self.w as i32 }
    fn top(&self) -> i32 { self.y as i32 }
    fn bottom(&self) -> i32 { self.y as i32 + self.h as i32 }
    fn center_x(&self) -> i32 { self.left() + self.w as i32 / 2 }
    fn center_y(&self) -> i32 { self.top() + self.h as i32 / 2 }
}

// true if the [a_start, a_end) and [b_start, b_end) ranges intersect
fn ranges_overlap(a_start: i32, a_end: i32, b_start: i32, b_end: i32) -> bool {
    a_start < b_end && b_start < a_end
}

// shows/hides a TextArea's cursor. Hiding it means matching the cursor line's
// style so the cursor cell blends in, per ratatui-textarea's own convention.
// Shared so every TextArea - top-level or nested inside another widget like
// Reviews - hides its cursor the same way when it isn't the one being typed into.
pub fn set_cursor_visible(textarea: &mut TextArea, visible: bool) {
    if visible {
        textarea.set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
    } else {
        textarea.set_cursor_style(textarea.cursor_line_style());
    }
}

pub struct UINode<W: 'static> where for<'a> &'a W: Widget {
    widget: W,
    area: AreaDescription,
    up: RawUINode,
    right: RawUINode,
    down: RawUINode,
    left: RawUINode,
    // style the widget had before it was hovered, so unhover can restore it.
    // only ever populated by specializations that actually support hover styling.
    hover_base_style: Option<Style>,
}


impl<W: 'static> Deref for UINode<W> where for<'a> &'a W: Widget {
    type Target = W;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}


impl<W: 'static> DerefMut for UINode<W> where for<'a> &'a W: Widget {

    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}


impl<W: 'static> UINode<W> where for<'a> &'a W: Widget {
    pub fn new(widget: W, area: AreaDescription) -> Self {
        Self{
            widget,
            area,
            up: None,
            right: None,
            down: None,
            left: None,
            hover_base_style: None,
        }
    }
}

impl<W: 'static> DynUINode for UINode<W> where for<'a> &'a W: Widget {
    // a node with no special input handling has nothing to consume, so any
    // key (in particular Esc) just deselects it
    default fn handle_input(&mut self, key: KeyEvent) -> bool { key.code == KeyCode::Esc }

    default fn select(&mut self) {  }

    default fn deselect(&mut self) {  }

    default fn hover(&mut self) {  }

    default fn unhover(&mut self) {  }

    default fn up(&self) -> RawUINode {
        self.up.clone()
    }

    default fn left(&self) -> RawUINode {
        self.left.clone()
    }

    default fn down(&self) -> RawUINode {
        self.down.clone()
    }

    default fn right(&self) -> RawUINode {
        self.right.clone()
    }

    default fn set_up(&mut self, node: RawUINode) {
        self.up = node;
    }

    default fn set_right(&mut self, node: RawUINode) {
        self.right = node;
    }

    default fn set_down(&mut self, node: RawUINode) {
        self.down = node;
    }

    default fn set_left(&mut self, node: RawUINode) {
        self.left = node;
    }

    default fn set_area(&mut self, area: AreaDescription) {
        self.area = area;
    }

    default fn draw(&self, frame: &mut Frame) {
        let full = frame.area();
        let area = Rect {
            x: full.x + (full.width as f32 * (self.area.x as f32 / 100.0)) as u16,
            y: full.y + (full.height as f32 * (self.area.y as f32 / 100.0)) as u16,
            width: (full.width as f32 * (self.area.w as f32 / 100.0)) as u16,
            height: (full.height as f32 * (self.area.h as f32 / 100.0)) as u16,
        };

        frame.render_widget(&self.widget, area);
    }
}

// Only overrides the methods that actually need TextArea-specific behavior.
// Because the blanket `impl<W> DynUINode for UINode<W>` above marks its methods
// `default fn`, specialization lets this impl fall back to those defaults for
// everything it doesn't mention here (up/right/down/left) instead of requiring
// them to be reimplemented.
impl DynUINode for UINode<TextArea<'_>> {
    fn handle_input(&mut self, key: KeyEvent) -> bool {
        // Tab has no meaning inside a plain text area, so it deselects (like
        // Esc) rather than being typed in; UITree::handle_input then moves
        // on to the next widget in the same keystroke.
        if key.code == KeyCode::Esc || key.code == KeyCode::Tab {
            return true;
        }

        self.widget.input(key);

        false
    }

    fn select(&mut self) {
        set_cursor_visible(&mut self.widget, true);
    }

    fn deselect(&mut self) {
        set_cursor_visible(&mut self.widget, false);
    }

    fn hover(&mut self) {
        if self.hover_base_style.is_none() {
            if let Some(block) = self.widget.block() {
                self.hover_base_style = Some(Styled::style(block));
                let block = block.clone().fg(HOVER_COLOR);
                self.widget.set_block(block);
            }
        }
    }

    fn unhover(&mut self) {
        if let Some(base_style) = self.hover_base_style.take() {
            if let Some(block) = self.widget.block() {
                let block = block.clone().style(base_style);
                self.widget.set_block(block);
            }
        }
    }
}

// Reviews handles its own internal navigation (which review is highlighted,
// scroll position) once it's the active widget, so this just forwards input
// and reuses the same hover-whitens-the-border convention as TextArea above.
impl DynUINode for UINode<Reviews<'_>> {
    fn handle_input(&mut self, key: KeyEvent) -> bool {
        self.widget.handle_input(key)
    }

    fn select(&mut self) {
        self.widget.select();
        self.widget.set_block(self.widget.block().clone().title_style(Style::default().light_blue()))
    }

    fn deselect(&mut self) {
        self.widget.deselect();
        self.widget.set_block(self.widget.block().clone().title_style(Style::default().dark_gray()))
    }

    fn hover(&mut self) {
        if self.hover_base_style.is_none() {
            self.hover_base_style = Some(Styled::style(self.widget.block()));
            let block = self.widget.block().clone().fg(HOVER_COLOR);
            self.widget.set_block(block);
        }
    }

    fn unhover(&mut self) {
        if let Some(base_style) = self.hover_base_style.take() {
            let block = self.widget.block().clone().style(base_style);
            self.widget.set_block(block);
        }
    }
}

pub struct UITree {
    // the currently selected item
    root: RawUINode,
    using_selection: bool
}

impl UITree {
    pub fn new(root: RawUINode) -> Self {
        if let Some(root) = &root {
            root.borrow_mut().hover();
        }

        Self{
            root,
            using_selection: false
        }
    }

    // traverses every node reachable from root via up/right/down/left and draws it
    pub fn draw(&self, frame: &mut Frame) {
        let Some(root) = &self.root else { return };

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        visited.insert(Rc::as_ptr(root) as *const () as usize);
        queue.push_back(root.clone());

        while let Some(node) = queue.pop_front() {
            let node_ref = node.borrow();
            node_ref.draw(frame);

            for neighbor in [node_ref.up(), node_ref.right(), node_ref.down(), node_ref.left()].into_iter().flatten() {
                if visited.insert(Rc::as_ptr(&neighbor) as *const () as usize) {
                    queue.push_back(neighbor);
                }
            }
        }
    }

    pub fn set_root(&mut self, node: RawUINode) {
        if let Some(old) = &self.root {
            old.borrow_mut().unhover();
        }
        if let Some(new) = &node {
            new.borrow_mut().hover();
        }
        self.root = node;
    }

    // moves the selected/hovered node in a direction, transferring the hover
    // highlight from the old node to the new one if there is a neighbor there
    fn navigate(&mut self, dir: impl Fn(&dyn DynUINode) -> RawUINode) {
        let node = self.root.as_ref().and_then(|root| dir(&*root.borrow()));

        if let Some(node) = node {
            if let Some(old) = &self.root {
                old.borrow_mut().unhover();
            }
            node.borrow_mut().hover();
            self.root = Some(node);
        }
    }

    pub fn handle_input(&mut self, input: KeyEvent) -> bool {
        if self.using_selection {
            if !input.is_release() {
                let should_deselect = self.root.as_ref()
                    .map(|root| root.borrow_mut().handle_input(input))
                    .unwrap_or(false);

                if should_deselect {
                    self.using_selection = false;
                    if let Some(root) = &self.root {
                        root.borrow_mut().deselect();
                    }

                    // a widget that has no "next" concept of its own (e.g. a
                    // plain text area) just deselects on Tab, same as Esc;
                    // one whose handle_input consumes Tab itself (e.g.
                    // Reviews, moving between its own review cells) never
                    // reports should_deselect for it, so this doesn't run.
                    if input.code == KeyCode::Tab {
                        self.tab_next();
                    }
                }
            }
        } else {
            if input.is_press() {
                match input.code {
                    KeyCode::Enter => {
                        self.using_selection = true;
                        if let Some(root) = &self.root {
                            root.borrow_mut().select();
                        }
                    }
                    KeyCode::Left => self.navigate(|n| n.left()),
                    KeyCode::Right => self.navigate(|n| n.right()),
                    KeyCode::Up => self.navigate(|n| n.up()),
                    KeyCode::Down => self.navigate(|n| n.down()),
                    KeyCode::Tab => self.tab_next(),
                    KeyCode::Char(c) if c.to_ascii_lowercase() == 'q' => return true,
                    _ => {}
                }
            }
        }
        false
    }

    // moves the hovered node to the "next" widget: right if there's a
    // neighbor there, else down and then as far left as possible, else (no
    // right or down neighbor - we're at the bottom-right of the layout)
    // wraps around by going as far up and then as far left as possible.
    fn tab_next(&mut self) {
        let Some(current) = self.root.clone() else { return };

        let right = current.borrow().right();
        let next = if let Some(right) = right {
            right
        } else if let Some(down) = current.borrow().down() {
            Self::furthest(down, |n| n.left())
        } else {
            let top = Self::furthest(current.clone(), |n| n.up());
            Self::furthest(top, |n| n.left())
        };

        if let Some(old) = &self.root {
            old.borrow_mut().unhover();
        }
        next.borrow_mut().hover();
        self.root = Some(next);
    }

    // follows a direction from `start` repeatedly until it has no neighbor
    // left in that direction, returning the last node reached.
    fn furthest(start: ValidUINode, dir: impl Fn(&dyn DynUINode) -> RawUINode) -> ValidUINode {
        let mut node = start;
        loop {
            let next = dir(&*node.borrow());
            match next {
                Some(n) => node = n,
                None => return node,
            }
        }
    }
}

// Builds a UITree from raw widgets placed at caller-chosen AreaDescriptions,
// without the caller ever touching Rc<RefCell<..>> directly. `build` figures
// out each widget's up/right/down/left neighbors from the geometry: for a
// given direction, it first looks for the closest widget that starts beyond
// the node's edge in that direction *and* overlaps it on the other axis (e.g.
// "right" candidates must start at/after this node's right edge and share
// some vertical range with it) - this is what makes aligned rows/columns link
// up the way you'd expect. If nothing satisfies that, it falls back to the
// closest widget whose center simply lies in that direction, so irregular or
// staggered layouts still get *something* reasonable instead of a dead end.
// A typed, cloneable handle to a widget added to a UITreeBuilder/UITree,
// returned by `add` alongside the type-erased node pushed into the tree.
// Lets the caller reach back into a specific widget (e.g. to read out its
// final contents after the input loop exits) without downcasting through
// the tree's `dyn DynUINode` nodes.
pub struct NodeHandle<W: 'static>(Rc<RefCell<UINode<W>>>) where for<'a> &'a W: Widget;

impl<W: 'static> NodeHandle<W> where for<'a> &'a W: Widget {
    pub fn borrow(&self) -> Ref<'_, W> {
        Ref::map(self.0.borrow(), |n| &**n)
    }

    pub fn borrow_mut(&self) -> RefMut<'_, W> {
        RefMut::map(self.0.borrow_mut(), |n| &mut **n)
    }
}

pub struct UITreeBuilder {
    nodes: Vec<(ValidUINode, AreaDescription)>,
}

impl UITreeBuilder {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn add<W: 'static>(&mut self, widget: W, area: AreaDescription) -> NodeHandle<W> where for<'a> &'a W: Widget {
        let node = Rc::new(RefCell::new(UINode::new(widget, area)));
        node.borrow_mut().deselect();
        self.nodes.push((node.clone() as ValidUINode, area));
        NodeHandle(node)
    }

    pub fn build(self) -> UITree {
        for i in 0..self.nodes.len() {
            let up = self.nearest(i,
                |a, b| ranges_overlap(a.left(), a.right(), b.left(), b.right()) && b.bottom() <= a.top(),
                |a, b| (a.top() - b.bottom()) as i64,
                |a, b| b.center_y() < a.center_y(),
            );
            let down = self.nearest(i,
                |a, b| ranges_overlap(a.left(), a.right(), b.left(), b.right()) && b.top() >= a.bottom(),
                |a, b| (b.top() - a.bottom()) as i64,
                |a, b| b.center_y() > a.center_y(),
            );
            let left = self.nearest(i,
                |a, b| ranges_overlap(a.top(), a.bottom(), b.top(), b.bottom()) && b.right() <= a.left(),
                |a, b| (a.left() - b.right()) as i64,
                |a, b| b.center_x() < a.center_x(),
            );
            let right = self.nearest(i,
                |a, b| ranges_overlap(a.top(), a.bottom(), b.top(), b.bottom()) && b.left() >= a.right(),
                |a, b| (b.left() - a.right()) as i64,
                |a, b| b.center_x() > a.center_x(),
            );

            let node = &self.nodes[i].0;
            node.borrow_mut().set_up(up.map(|j| self.nodes[j].0.clone()));
            node.borrow_mut().set_down(down.map(|j| self.nodes[j].0.clone()));
            node.borrow_mut().set_left(left.map(|j| self.nodes[j].0.clone()));
            node.borrow_mut().set_right(right.map(|j| self.nodes[j].0.clone()));

            // every widget starts deselected; without this the cursor of
            // whichever widget happens to never be visited would stay
            // visible forever, since deselect() is only otherwise triggered
            // by leaving selection mode.
            node.borrow_mut().deselect();
        }

        let root = self.nodes.into_iter().next().map(|(node, _)| node);
        UITree::new(root)
    }

    // finds the index of the node closest to `self.nodes[self_index]` in one
    // direction. `is_aligned` + `aligned_score` are tried first (requires
    // perpendicular-axis overlap); `fallback_is_beyond` is used only if no
    // node satisfies the aligned check, scored by center-to-center distance.
    fn nearest(
        &self,
        self_index: usize,
        is_aligned: impl Fn(&AreaDescription, &AreaDescription) -> bool,
        aligned_score: impl Fn(&AreaDescription, &AreaDescription) -> i64,
        fallback_is_beyond: impl Fn(&AreaDescription, &AreaDescription) -> bool,
    ) -> Option<usize> {
        let a = &self.nodes[self_index].1;

        let others = || self.nodes.iter().enumerate().filter(move |(j, _)| *j != self_index);

        others()
            .filter(|(_, (_, b))| is_aligned(a, b))
            .min_by_key(|(_, (_, b))| aligned_score(a, b))
            .or_else(|| {
                others()
                    .filter(|(_, (_, b))| fallback_is_beyond(a, b))
                    .min_by_key(|(_, (_, b))| {
                        let dx = (a.center_x() - b.center_x()) as i64;
                        let dy = (a.center_y() - b.center_y()) as i64;
                        dx * dx + dy * dy
                    })
            })
            .map(|(j, _)| j)
    }
}