use gtk::cairo;
use gtk::gdk;
use gtk::prelude::*;
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::f64::consts::TAU;
use std::rc::Rc;

use crate::client::{GraphEdge, GraphLayout, GraphNeighborhood, GraphNode};

const NODE_RADIUS: f64 = 12.0;
const VIEW_PADDING: f64 = 24.0;

type NodeSelectedCallback = Rc<RefCell<Option<Box<dyn Fn(&str)>>>>;
type NodeActivatedCallback = Rc<RefCell<Option<Box<dyn Fn(&str)>>>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphScope {
    Vault,
    Neighborhood,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphLoadState {
    Idle,
    Loading,
    Ready,
    Empty,
    Error(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphScene {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub selected_path: Option<String>,
    pub load_state: GraphLoadState,
}

impl GraphScene {
    pub fn idle() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            selected_path: None,
            load_state: GraphLoadState::Idle,
        }
    }

    pub fn loading(selected_path: Option<String>) -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            selected_path,
            load_state: GraphLoadState::Loading,
        }
    }

    pub fn error(message: impl Into<String>, selected_path: Option<String>) -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            selected_path,
            load_state: GraphLoadState::Error(message.into()),
        }
    }

    pub fn ready(
        nodes: Vec<GraphNode>,
        edges: Vec<GraphEdge>,
        selected_path: Option<String>,
    ) -> Self {
        let load_state = if nodes.is_empty() {
            GraphLoadState::Empty
        } else {
            GraphLoadState::Ready
        };
        Self {
            nodes,
            edges,
            selected_path,
            load_state,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphContextDetails {
    pub selected_path: Option<String>,
    pub selected_label: Option<String>,
    pub neighbors: Vec<String>,
    pub backlinks: Vec<String>,
}

impl GraphContextDetails {
    pub fn empty() -> Self {
        Self {
            selected_path: None,
            selected_label: None,
            neighbors: Vec::new(),
            backlinks: Vec::new(),
        }
    }
}

#[derive(Clone)]
pub struct GraphView {
    widget: gtk::Box,
    status_label: gtk::Label,
    drawing_area: gtk::DrawingArea,
    scene: Rc<RefCell<GraphScene>>,
    on_node_selected: NodeSelectedCallback,
    on_node_activated: NodeActivatedCallback,
}

fn basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn label_for_path(path: &str) -> String {
    basename(path).trim_end_matches(".md").to_string()
}

fn layout_bounds(nodes: &[GraphNode]) -> Option<(f64, f64, f64, f64)> {
    let first = nodes.first()?;
    let mut min_x = first.x;
    let mut max_x = first.x;
    let mut min_y = first.y;
    let mut max_y = first.y;
    for node in nodes.iter().skip(1) {
        min_x = min_x.min(node.x);
        max_x = max_x.max(node.x);
        min_y = min_y.min(node.y);
        max_y = max_y.max(node.y);
    }
    Some((min_x, max_x, min_y, max_y))
}

fn graph_transform(nodes: &[GraphNode], width: f64, height: f64) -> (f64, f64, f64) {
    if width <= (VIEW_PADDING * 2.0) || height <= (VIEW_PADDING * 2.0) {
        return (1.0, VIEW_PADDING, VIEW_PADDING);
    }

    let Some((min_x, max_x, min_y, max_y)) = layout_bounds(nodes) else {
        return (1.0, VIEW_PADDING, VIEW_PADDING);
    };

    let graph_width = (max_x - min_x).max(1.0);
    let graph_height = (max_y - min_y).max(1.0);
    let available_width = (width - (VIEW_PADDING * 2.0)).max(1.0);
    let available_height = (height - (VIEW_PADDING * 2.0)).max(1.0);
    let scale = (available_width / graph_width)
        .min(available_height / graph_height)
        .max(0.1);
    let offset_x =
        VIEW_PADDING + ((available_width - (graph_width * scale)) / 2.0) - (min_x * scale);
    let offset_y =
        VIEW_PADDING + ((available_height - (graph_height * scale)) / 2.0) - (min_y * scale);
    (scale, offset_x, offset_y)
}

fn transform_scene_point(
    node: &GraphNode,
    scene: &GraphScene,
    width: f64,
    height: f64,
) -> (f64, f64) {
    let (scale, offset_x, offset_y) = graph_transform(&scene.nodes, width, height);
    (node.x * scale + offset_x, node.y * scale + offset_y)
}

fn hit_test_node(scene: &GraphScene, width: f64, height: f64, x: f64, y: f64) -> Option<String> {
    let mut best: Option<(String, f64)> = None;
    for node in &scene.nodes {
        let (screen_x, screen_y) = transform_scene_point(node, scene, width, height);
        let distance = ((screen_x - x).powi(2) + (screen_y - y).powi(2)).sqrt();
        if distance <= NODE_RADIUS * 1.5 {
            match &best {
                Some((_, best_distance)) if distance >= *best_distance => {}
                _ => best = Some((node.id.clone(), distance)),
            }
        }
    }
    best.map(|(path, _)| path)
}

fn draw_scene(scene: &GraphScene, cr: &cairo::Context, width: f64, height: f64) {
    cr.set_source_rgb(0.98, 0.98, 0.98);
    let _ = cr.paint();

    if scene.nodes.is_empty() {
        return;
    }

    let (scale, offset_x, offset_y) = graph_transform(&scene.nodes, width, height);
    let node_index: HashMap<&str, &GraphNode> = scene
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect();

    cr.set_line_width(1.0);
    cr.set_source_rgba(0.45, 0.48, 0.52, 0.5);
    for edge in &scene.edges {
        let Some(source) = node_index.get(edge.source.as_str()) else {
            continue;
        };
        let Some(target) = node_index.get(edge.target.as_str()) else {
            continue;
        };
        cr.move_to((source.x * scale) + offset_x, (source.y * scale) + offset_y);
        cr.line_to((target.x * scale) + offset_x, (target.y * scale) + offset_y);
        let _ = cr.stroke();
    }

    for node in &scene.nodes {
        let screen_x = (node.x * scale) + offset_x;
        let screen_y = (node.y * scale) + offset_y;
        let selected = scene.selected_path.as_deref() == Some(node.id.as_str());

        if selected {
            cr.set_source_rgb(0.10, 0.35, 0.75);
        } else {
            cr.set_source_rgb(0.22, 0.22, 0.24);
        }
        cr.arc(screen_x, screen_y, NODE_RADIUS, 0.0, TAU);
        let _ = cr.fill();

        cr.set_source_rgb(1.0, 1.0, 1.0);
        cr.select_font_face("Sans", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
        cr.set_font_size(11.0);
        cr.move_to(screen_x + (NODE_RADIUS + 6.0), screen_y + 4.0);
        let _ = cr.show_text(&node.label);
    }
}

pub fn normalize_vault_layout(layout: GraphLayout) -> GraphScene {
    GraphScene::ready(layout.nodes, layout.edges, None)
}

pub fn normalize_neighborhood_layout(
    neighborhood: GraphNeighborhood,
    vault_layout: Option<&GraphLayout>,
    selected_path: Option<&str>,
) -> GraphScene {
    let vault_nodes = vault_layout.map(|layout| {
        layout
            .nodes
            .iter()
            .map(|node| (node.id.as_str(), node))
            .collect::<HashMap<_, _>>()
    });
    let mut nodes = Vec::new();
    let count = neighborhood.nodes.len();

    for (index, path) in neighborhood.nodes.iter().enumerate() {
        if let Some(vault_node) = vault_nodes
            .as_ref()
            .and_then(|nodes| nodes.get(path.as_str()))
        {
            nodes.push((*vault_node).clone());
            continue;
        }

        let angle = if count == 0 {
            0.0
        } else {
            (index as f64 / count as f64) * TAU
        };
        let radius = 160.0;
        nodes.push(GraphNode {
            id: path.clone(),
            label: label_for_path(path),
            x: radius * angle.cos(),
            y: radius * angle.sin(),
        });
    }

    GraphScene::ready(
        nodes,
        neighborhood.edges,
        selected_path.map(ToOwned::to_owned),
    )
}

pub fn graph_context_details(scene: &GraphScene) -> GraphContextDetails {
    let Some(selected_path) = scene.selected_path.as_deref() else {
        return GraphContextDetails::empty();
    };
    let selected_label = scene
        .nodes
        .iter()
        .find(|node| node.id == selected_path)
        .map(|node| node.label.clone())
        .or_else(|| Some(label_for_path(selected_path)));

    let mut neighbors = BTreeSet::new();
    let mut backlinks = BTreeSet::new();
    for edge in &scene.edges {
        if edge.source == selected_path {
            neighbors.insert(edge.target.clone());
        }
        if edge.target == selected_path {
            backlinks.insert(edge.source.clone());
        }
    }

    GraphContextDetails {
        selected_path: Some(selected_path.to_string()),
        selected_label,
        neighbors: neighbors.into_iter().collect(),
        backlinks: backlinks.into_iter().collect(),
    }
}

pub fn graph_status_text(scope: GraphScope, state: &GraphLoadState) -> String {
    match state {
        GraphLoadState::Idle => "Graph is idle".to_string(),
        GraphLoadState::Loading => match scope {
            GraphScope::Vault => "Loading vault graph...".to_string(),
            GraphScope::Neighborhood => "Loading graph neighborhood...".to_string(),
        },
        GraphLoadState::Ready => match scope {
            GraphScope::Vault => "Vault graph".to_string(),
            GraphScope::Neighborhood => "Node neighborhood".to_string(),
        },
        GraphLoadState::Empty => match scope {
            GraphScope::Vault => "Vault graph is empty".to_string(),
            GraphScope::Neighborhood => "No neighboring notes found".to_string(),
        },
        GraphLoadState::Error(message) => format!("Graph error: {message}"),
    }
}

impl GraphView {
    pub fn new() -> Self {
        let widget = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(8)
            .margin_top(12)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .build();
        widget.set_widget_name("knot.content.graph");

        let status_label = gtk::Label::builder()
            .label("Graph is idle")
            .xalign(0.0)
            .css_classes(vec!["dim-label".to_string()])
            .build();

        let drawing_area = gtk::DrawingArea::builder()
            .content_width(900)
            .content_height(600)
            .hexpand(true)
            .vexpand(true)
            .build();
        drawing_area.set_widget_name("knot.graph.canvas");
        let scrolled = gtk::ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(true)
            .child(&drawing_area)
            .build();

        widget.append(&status_label);
        widget.append(&scrolled);

        let scene = Rc::new(RefCell::new(GraphScene::idle()));
        let on_node_selected: NodeSelectedCallback = Rc::new(RefCell::new(None));
        let on_node_activated: NodeActivatedCallback = Rc::new(RefCell::new(None));

        drawing_area.set_draw_func({
            let scene = Rc::clone(&scene);
            move |_area, cr, width, height| {
                draw_scene(&scene.borrow(), cr, width as f64, height as f64);
            }
        });

        let click = gtk::GestureClick::new();
        click.set_button(gdk::ffi::GDK_BUTTON_PRIMARY as u32);
        click.connect_pressed({
            let scene = Rc::clone(&scene);
            let drawing_area = drawing_area.clone();
            let on_node_selected = Rc::clone(&on_node_selected);
            let on_node_activated = Rc::clone(&on_node_activated);
            move |_gesture, n_press, x, y| {
                let width = drawing_area.width() as f64;
                let height = drawing_area.height() as f64;
                let selected_path = {
                    let mut scene = scene.borrow_mut();
                    let Some(path) = hit_test_node(&scene, width, height, x, y) else {
                        return;
                    };
                    scene.selected_path = Some(path.clone());
                    path
                };

                drawing_area.queue_draw();
                if let Some(callback) = &*on_node_selected.borrow() {
                    callback(&selected_path);
                }
                if n_press >= 2 {
                    if let Some(callback) = &*on_node_activated.borrow() {
                        callback(&selected_path);
                    }
                }
            }
        });
        drawing_area.add_controller(click);

        Self {
            widget,
            status_label,
            drawing_area,
            scene,
            on_node_selected,
            on_node_activated,
        }
    }

    pub fn set_scene(&self, scope: GraphScope, scene: GraphScene) {
        self.status_label
            .set_label(&graph_status_text(scope, &scene.load_state));
        *self.scene.borrow_mut() = scene;
        self.drawing_area.queue_draw();
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.widget
    }

    pub fn connect_node_selected<F>(&self, f: F)
    where
        F: Fn(&str) + 'static,
    {
        *self.on_node_selected.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_node_activated<F>(&self, f: F)
    where
        F: Fn(&str) + 'static,
    {
        *self.on_node_activated.borrow_mut() = Some(Box::new(f));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_vault_layout() -> GraphLayout {
        GraphLayout {
            nodes: vec![
                GraphNode {
                    id: "notes/one.md".to_string(),
                    label: "one".to_string(),
                    x: 0.0,
                    y: 0.0,
                },
                GraphNode {
                    id: "notes/two.md".to_string(),
                    label: "two".to_string(),
                    x: 100.0,
                    y: 0.0,
                },
                GraphNode {
                    id: "notes/three.md".to_string(),
                    label: "three".to_string(),
                    x: 50.0,
                    y: 80.0,
                },
            ],
            edges: vec![
                GraphEdge {
                    source: "notes/one.md".to_string(),
                    target: "notes/two.md".to_string(),
                },
                GraphEdge {
                    source: "notes/three.md".to_string(),
                    target: "notes/one.md".to_string(),
                },
            ],
        }
    }

    #[test]
    fn normalize_vault_layout_keeps_backend_positions() {
        let scene = normalize_vault_layout(sample_vault_layout());

        assert_eq!(scene.load_state, GraphLoadState::Ready);
        assert_eq!(scene.nodes.len(), 3);
        assert_eq!(scene.edges.len(), 2);
        assert_eq!(scene.nodes[1].x, 100.0);
    }

    #[test]
    fn normalize_neighborhood_layout_reuses_vault_positions_when_available() {
        let scene = normalize_neighborhood_layout(
            GraphNeighborhood {
                nodes: vec!["notes/one.md".to_string(), "notes/two.md".to_string()],
                edges: vec![GraphEdge {
                    source: "notes/one.md".to_string(),
                    target: "notes/two.md".to_string(),
                }],
            },
            Some(&sample_vault_layout()),
            Some("notes/one.md"),
        );

        assert_eq!(scene.selected_path.as_deref(), Some("notes/one.md"));
        assert_eq!(scene.nodes[1].x, 100.0);
        assert_eq!(scene.nodes[1].label, "two");
    }

    #[test]
    fn normalize_neighborhood_layout_falls_back_to_generated_positions() {
        let scene = normalize_neighborhood_layout(
            GraphNeighborhood {
                nodes: vec!["notes/a.md".to_string(), "notes/b.md".to_string()],
                edges: Vec::new(),
            },
            None,
            None,
        );

        assert_eq!(scene.load_state, GraphLoadState::Ready);
        assert_ne!(scene.nodes[0].x, scene.nodes[1].x);
        assert_eq!(scene.nodes[0].label, "a");
    }

    #[test]
    fn graph_context_details_separate_neighbors_and_backlinks() {
        let mut scene = normalize_vault_layout(sample_vault_layout());
        scene.selected_path = Some("notes/one.md".to_string());

        let details = graph_context_details(&scene);

        assert_eq!(details.selected_label.as_deref(), Some("one"));
        assert_eq!(details.neighbors, vec!["notes/two.md".to_string()]);
        assert_eq!(details.backlinks, vec!["notes/three.md".to_string()]);
    }

    #[test]
    fn hit_test_node_matches_transformed_positions() {
        let scene = normalize_vault_layout(sample_vault_layout());
        let target = &scene.nodes[0];
        let (x, y) = transform_scene_point(target, &scene, 800.0, 600.0);

        assert_eq!(
            hit_test_node(&scene, 800.0, 600.0, x, y).as_deref(),
            Some("notes/one.md")
        );
    }
}
