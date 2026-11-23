//! The dungeon as it is built: rooms in the order they were added, each with one link per exit
//! of its piece (or none yet). A link is two-way; the plan is a tree.

/// What an exit of a node is joined to: the other node and its exit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Link {
    pub to: usize,
    pub to_exit: usize,
}

pub(super) struct Node {
    /// Index into the pool's pieces (and the attempt's oriented pieces).
    pub piece: usize,
    pub parent: Option<usize>,
    /// One entry per exit of the piece, `None` while the exit is free.
    pub links: Vec<Option<Link>>,
}

pub(super) struct Tree {
    pub nodes: Vec<Node>,
    /// The main path, start first and boss last once it is built.
    pub spine: Vec<usize>,
}

impl Tree {
    /// A tree of one room, the start, with `exits` free exits.
    pub fn new(piece: usize, exits: usize) -> Self {
        Self {
            nodes: vec![Node {
                piece,
                parent: None,
                links: vec![None; exits],
            }],
            spine: vec![0],
        }
    }

    /// Adds a room with `exits` exits, joined to free exit `host_exit` of `host` by its exit
    /// `exit`. Returns its index.
    pub fn attach(
        &mut self,
        host: usize,
        host_exit: usize,
        piece: usize,
        exit: usize,
        exits: usize,
    ) -> usize {
        let node = self.nodes.len();
        let mut links = vec![None; exits];
        links[exit] = Some(Link {
            to: host,
            to_exit: host_exit,
        });
        self.nodes.push(Node {
            piece,
            parent: Some(host),
            links,
        });
        self.nodes[host].links[host_exit] = Some(Link {
            to: node,
            to_exit: exit,
        });
        node
    }

    /// Takes back the last room added (never the start).
    pub fn detach_last(&mut self) {
        let Some(node) = self.nodes.pop() else {
            return;
        };
        let removed = self.nodes.len();
        if let Some(host) = node.parent {
            for link in &mut self.nodes[host].links {
                if link.is_some_and(|link| link.to == removed) {
                    *link = None;
                }
            }
        }
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// The exits of `node` that lead nowhere yet.
    pub fn free_exits(&self, node: usize) -> Vec<usize> {
        (0..self.nodes[node].links.len())
            .filter(|exit| self.nodes[node].links[*exit].is_none())
            .collect()
    }

    pub fn uses(&self, piece: usize) -> bool {
        self.nodes.iter().any(|node| node.piece == piece)
    }

    /// The exit of `node` that leads to its parent.
    pub fn parent_exit(&self, node: usize) -> Option<usize> {
        let parent = self.nodes[node].parent?;
        self.exit_toward(node, parent)
    }

    /// The exit of `from` that leads to `to`.
    pub fn exit_toward(&self, from: usize, to: usize) -> Option<usize> {
        self.nodes[from]
            .links
            .iter()
            .position(|link| link.is_some_and(|link| link.to == to))
    }

    /// The rooms below `node`, as `(exit of node, child)`.
    pub fn children(&self, node: usize) -> Vec<(usize, usize)> {
        self.nodes[node]
            .links
            .iter()
            .enumerate()
            .filter_map(|(exit, link)| {
                let link = (*link)?;
                (self.nodes[link.to].parent == Some(node)).then_some((exit, link.to))
            })
            .collect()
    }

    /// `node` and every room below it.
    pub fn subtree(&self, node: usize) -> Vec<bool> {
        let mut inside = vec![false; self.nodes.len()];
        let mut pending = vec![node];
        while let Some(room) = pending.pop() {
            if std::mem::replace(&mut inside[room], true) {
                continue;
            }
            pending.extend(self.children(room).into_iter().map(|(_, child)| child));
        }
        inside
    }

    /// The last room of the spine: the newest while the spine grows, the boss room once it is
    /// built.
    pub fn boss(&self) -> usize {
        self.spine.last().copied().unwrap_or(0)
    }
}
