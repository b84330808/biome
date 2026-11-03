//! The syntax nodes rows of the model stand for.
//!
//! The model is shared between the analyzers of the host document and of each
//! embedded snippet, which run on trees of different languages. A row that
//! stands for a node names it with a [`NodeId`] of the node's language, and
//! only an analysis of the tree holding the node can turn the id into the
//! node: [`Nodes`] is built from the [`AnalyzedTree`] an analyzer gives its
//! visitors, which can't be made from a root alone. A rule of the analyzer of
//! an HTML document therefore can't resolve a JavaScript node of a snippet,
//! and the reverse.

use super::model::{SnippetId, VueSemanticModel};
use biome_analyze::AnalyzedTree;
use biome_html_syntax::{HtmlLanguage, HtmlRoot};
use biome_js_syntax::{AnyJsRoot, JsLanguage};
use biome_rowan::{AstNode, Language, SyntaxNode};
use std::marker::PhantomData;

/// Identifies a syntax node of language `L` that a row of the model stands
/// for.
///
/// Ids are dense over the whole model, so [`Nodes`] finds a node in constant
/// time.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NodeId<L: Language> {
    index: u32,
    language: PhantomData<L>,
}

impl<L: Language> NodeId<L> {
    pub(crate) fn new(index: usize) -> Self {
        Self {
            index: index as u32,
            language: PhantomData,
        }
    }

    pub(crate) fn index(self) -> usize {
        self.index as usize
    }
}

/// Identifies a JavaScript syntax node that a row of the model stands for.
pub type JsNodeId = NodeId<JsLanguage>;

/// Identifies a syntax node of the host document that a row of the model
/// stands for.
pub type HtmlNodeId = NodeId<HtmlLanguage>;

/// The nodes of one analyzed tree that rows of the model stand for.
///
/// Rows written in another tree, such as another script block of the same
/// single-file component, have no node here.
///
/// Code that holds a root without the [`AnalyzedTree`] of its analysis
/// cannot resolve nodes:
///
/// ```compile_fail
/// # use biome_js_syntax::AnyJsRoot;
/// # use biome_vue_semantic::VueSemanticModel;
/// fn leak(model: &VueSemanticModel, root: &AnyJsRoot) {
///     let _ = model.resolve_js_nodes(root, None);
/// }
/// ```
///
/// Nor can an id be resolved against the nodes of another language:
///
/// ```compile_fail
/// # use biome_html_syntax::HtmlLanguage;
/// # use biome_vue_semantic::{JsNodeId, Nodes};
/// fn cross(nodes: &Nodes<HtmlLanguage>, id: JsNodeId) {
///     let _ = nodes.get(id);
/// }
/// ```
///
/// ```compile_fail
/// # use biome_analyze::AnalyzedTree;
/// # use biome_html_syntax::HtmlLanguage;
/// # use biome_vue_semantic::VueSemanticModel;
/// fn cross(model: &VueSemanticModel, tree: AnalyzedTree<'_, HtmlLanguage>) {
///     let _ = model.js_nodes(tree, None);
/// }
/// ```
#[derive(Debug)]
pub struct Nodes<L: Language> {
    nodes: Box<[Option<SyntaxNode<L>>]>,
}

impl<L: Language> Nodes<L> {
    /// Returns the node `id` names, or `None` when it is not in the tree
    /// these nodes were resolved in.
    pub fn get(&self, id: NodeId<L>) -> Option<&SyntaxNode<L>> {
        self.nodes.get(id.index())?.as_ref()
    }
}

impl VueSemanticModel {
    /// Resolves the JavaScript nodes rows of the model stand for in `tree`.
    ///
    /// `snippet` is the snippet of a single-file component that `tree` is,
    /// or `None` when `tree` is a whole JavaScript file. The pointers of the
    /// rows written in that tree are resolved once, so that the node of a
    /// [`JsNodeId`] is then found in constant time.
    pub fn js_nodes(
        &self,
        tree: AnalyzedTree<'_, JsLanguage>,
        snippet: Option<SnippetId>,
    ) -> Nodes<JsLanguage> {
        self.resolve_js_nodes(tree.root(), snippet)
    }

    /// Resolves the JavaScript nodes of the tree of `snippet`, rooted at
    /// `root`.
    ///
    /// Only the analyzer of `root` may see its nodes, so callers outside this
    /// crate go through [`Self::js_nodes`].
    pub(crate) fn resolve_js_nodes(
        &self,
        root: &AnyJsRoot,
        snippet: Option<SnippetId>,
    ) -> Nodes<JsLanguage> {
        let nodes = self
            .data
            .js_nodes
            .iter()
            .map(|node| {
                (node.snippet == snippet)
                    .then(|| node.ptr.try_to_node(root.syntax()))
                    .flatten()
            })
            .collect();
        Nodes { nodes }
    }

    /// Resolves the nodes of the host document that rows of the model stand
    /// for in `tree`, the tree of the host document.
    ///
    /// The pointers are resolved once, so that the node of an [`HtmlNodeId`]
    /// is then found in constant time.
    pub fn html_nodes(&self, tree: AnalyzedTree<'_, HtmlLanguage>) -> Nodes<HtmlLanguage> {
        self.resolve_html_nodes(tree.root())
    }

    /// Resolves the nodes of the host document rooted at `root`.
    ///
    /// Only the analyzer of `root` may see its nodes, so callers outside this
    /// crate go through [`Self::html_nodes`].
    pub(crate) fn resolve_html_nodes(&self, root: &HtmlRoot) -> Nodes<HtmlLanguage> {
        let nodes = self
            .data
            .html_nodes
            .iter()
            .map(|ptr| ptr.try_to_node(root.syntax()))
            .collect();
        Nodes { nodes }
    }
}
