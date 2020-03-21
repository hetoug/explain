pub(crate) fn dot(explain: &crate::Explain) -> String {
    Graph::from(explain).render()
}

type Nd = usize;
type Ed<'a> = &'a (usize, usize);

struct Graph {
    nodes: Vec<Node>,
    edges: Vec<(usize, usize)>,
    current_id: usize,
    max_cost: f32,
}

impl Graph {
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            current_id: 0,
            max_cost: 0.,
        }
    }

    fn from(explain: &crate::Explain) -> Self {
        let mut graph = Self::new();

        graph.plan(&explain.plan, None, &explain.plan);

        graph
    }

    fn plan(&mut self, root: &crate::Plan, root_id: Option<usize>, plan: &crate::Plan) {
        let id = self.current_id;
        self.current_id += 1;

        let node = Node::from(root, plan);
        if node.cost > self.max_cost {
            self.max_cost = node.cost;
        }
        self.nodes.push(node);

        if let Some(root_id) = root_id {
            self.edges.push((root_id, id));
        }

        for child in &plan.plans {
            self.plan(root, Some(id), child);
        }
    }

    fn render(&self) -> String {
        let mut output = Vec::new();

        dot::render(self, &mut output).unwrap();

        std::str::from_utf8(&output).unwrap().to_string()
    }

    fn node(&self, n: Nd) -> Option<&Node> {
        self.nodes.get(n)
    }
}

struct Node {
    n_workers: usize,
    info: String,
    rows: u32,
    time: String,
    cost: f32,
    ty: String,
}

impl Node {
    fn from(root: &crate::Plan, plan: &crate::Plan) -> Self {
        let info = info(&plan);
        let cost = cost(&plan);
        let time = if let Some(time) = time(&plan) {
            let time_percent = (time / root.actual_total_time.unwrap() * 100.)
                .round()
                .trunc();

            if time < 1. {
                format!("<td>&lt; 1 ms | {} %</td>", time_percent)
            } else {
                let color = duration_color(time_percent);

                format!(
                    "<td bgcolor=\"{}\">{} ms | {} %</td>",
                    color,
                    time.round(),
                    time_percent
                )
            }
        } else {
            String::new()
        };

        Self {
            info,
            n_workers: plan.workers.len(),
            rows: plan.rows,
            time,
            ty: plan.node.to_string(),
            cost,
        }
    }
}

impl<'a> dot::Labeller<'a, Nd, Ed<'a>> for Graph {
    fn graph_id(&'a self) -> dot::Id<'a> {
        dot::Id::new("explain").unwrap()
    }

    fn node_id(&'a self, n: &Nd) -> dot::Id<'a> {
        dot::Id::new(format!("node{}", n)).unwrap()
    }

    fn node_label<'b>(&'b self, n: &Nd) -> dot::LabelText<'b> {
        let node = self.node(*n).unwrap();
        let percent = node.cost / self.max_cost;
        let color = color(percent);
        let bgcolor = if percent < 0.1 {
            String::new()
        } else if percent > 0.99 {
            format!("bgcolor=\"{}\"", color)
        } else {
            format!("bgcolor=\"{};{:.2}:white\"", color, percent)
        };

        let mut label = r#"<table border="0" cellborder="0" cellspacing="5">"#.to_string();
        label.push_str(&format!(
            r#"<tr><td align="left"><b>{}</b></td>{}</tr>"#,
            node.ty, node.time
        ));
        label.push_str(&format!(
            r#"<tr><td colspan="2" align="left">{}</td></tr>"#,
            node.info
        ));
        if node.n_workers > 0 {
            label.push_str(&format!(
                r#"<tr><td colspan="2" align="left">Workers: {}</td></tr>"#,
                node.n_workers
            ));
        }

        label.push_str(&format!(
            r#"<tr><td colspan="2" border="1" {}>Cost: {:.02}</td></tr>"#,
            bgcolor, node.cost
        ));
        label.push_str(&format!(
            r#"<tr><td colspan="2" align="left">Rows: {}</td></tr>"#,
            node.rows
        ));
        label.push_str("</table>");

        dot::LabelText::HtmlStr(label.into())
    }

    fn node_shape(&'a self, n: &Nd) -> Option<dot::LabelText<'a>> {
        let node = match self.node(*n) {
            Some(node) => node,
            None => return None,
        };

        let shape = if node.n_workers > 0 { "folder" } else { "box" };

        Some(dot::LabelText::LabelStr(shape.into()))
    }

    fn node_style(&'a self, _: &Nd) -> dot::Style {
        dot::Style::Rounded
    }

    fn kind(&self) -> dot::Kind {
        dot::Kind::Graph
    }
}

impl<'a> dot::GraphWalk<'a, Nd, Ed<'a>> for Graph {
    fn nodes(&self) -> dot::Nodes<'a, Nd> {
        (0..self.nodes.len()).collect()
    }

    fn edges(&'a self) -> dot::Edges<'a, Ed<'a>> {
        self.edges.iter().collect()
    }

    fn source(&self, e: &Ed<'_>) -> Nd {
        e.0
    }

    fn target(&self, e: &Ed<'_>) -> Nd {
        e.1
    }
}

fn duration_color(percent: f32) -> &'static str {
    if percent > 90. {
        "#880000"
    } else if percent > 40. {
        "#ee8800"
    } else if percent > 10. {
        "#fddb61"
    } else {
        "white"
    }
}

fn color(percent: f32) -> String {
    let hue = (100. - percent * 100.) * 1.2 / 360.;
    let (red, green, blue) = hsl_to_rgb(hue, 0.9, 0.4);

    format!("#{:02x}{:02x}{:02x}", red, green, blue)
}

fn hsl_to_rgb(hue: f32, saturation: f32, lightness: f32) -> (u8, u8, u8) {
    let (red, green, blue) = if saturation != 0. {
        let q = if lightness < 0.5 {
            lightness * (1. + saturation)
        } else {
            lightness + saturation - lightness * saturation
        };
        let p = 2. * lightness - q;

        (
            hue2rgb(p, q, hue + 1. / 3.),
            hue2rgb(p, q, hue),
            hue2rgb(p, q, hue - 1. / 3.),
        )
    } else {
        (lightness, lightness, lightness)
    };

    (
        (red * 255.) as u8,
        (green * 255.) as u8,
        (blue * 255.) as u8,
    )
}

fn hue2rgb(p: f32, q: f32, t: f32) -> f32 {
    let mut t = t;

    if t < 0. {
        t += 1.;
    }
    if t > 1. {
        t -= 1.;
    }
    if t < 1. / 6. {
        return p + (q - p) * 6. * t;
    }
    if t < 1. / 2. {
        return q;
    }
    if t < 2. / 3. {
        return p + (q - p) * (2. / 3. - t) * 6.;
    }

    p
}

fn info(plan: &crate::Plan) -> String {
    match &plan.node {
        crate::Node::Aggregate { keys, .. } => {
            if keys.is_empty() {
                String::new()
            } else {
                format!("by {}", keys.join(", "))
            }
        }
        crate::Node::HashJoin {
            join_type,
            hash_cond,
            ..
        } => format!("{} join on {}", join_type, hash_cond),
        crate::Node::Sort { keys, .. } => format!("by {}", keys.join(", ")),
        crate::Node::SeqScan { relation, .. } => format!("on {}", relation),
        _ => String::new(),
    }
}

fn time(plan: &crate::Plan) -> Option<f32> {
    if let Some(mut time) = plan.actual_total_time {
        for child in &plan.plans {
            time -= child.actual_total_time.unwrap_or_default();
        }

        Some(time)
    } else {
        None
    }
}

fn cost(plan: &crate::Plan) -> f32 {
    let mut cost = plan.total_cost;

    for child in &plan.plans {
        cost -= child.total_cost;
    }

    cost.max(0.)
}
