//! Radix tree for efficient prefix-based lookups.
//!
//! Provides O(k) lookup where k is the key length, with O(1) child access
//! via hash map indexing. Used by `PrefixMapMatcher` for longest-prefix-wins.

use std::collections::HashMap;

/// A radix tree (compressed trie) for efficient prefix lookups.
///
/// # Performance
///
/// - Insert: O(k) where k is key length
/// - Lookup: O(k) where k is key length
/// - Child access: O(1) via hash map
///
/// # Design (Envoy-inspired)
///
/// Uses node splitting to compress common prefixes, minimizing memory
/// and traversal depth. Each node stores its prefix edge and uses a
/// hash map for O(1) child lookup by first character.
#[derive(Debug, Clone)]
pub struct RadixTree<V> {
    root: Node<V>,
}

#[derive(Debug, Clone)]
struct Node<V> {
    /// The prefix edge leading to this node.
    prefix: String,
    /// Value stored at this node (if this is a "leaf" or intermediate with value).
    value: Option<V>,
    /// Children indexed by first character of their prefix.
    children: HashMap<u8, Node<V>>,
}

impl<V> Default for RadixTree<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V> RadixTree<V> {
    /// Create an empty radix tree.
    #[must_use]
    pub fn new() -> Self {
        Self {
            root: Node::new(String::new()),
        }
    }

    /// Insert a key-value pair.
    ///
    /// If the key already exists, the value is replaced and the old value returned.
    pub fn insert(&mut self, key: &str, value: V) -> Option<V> {
        self.root.insert(key, value)
    }

    /// Find the value for an exact key match.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&V> {
        self.root.get(key)
    }

    /// Find the value with the longest key that is a prefix of the input.
    ///
    /// # Example
    ///
    /// ```
    /// use rumi::RadixTree;
    ///
    /// let mut tree = RadixTree::new();
    /// tree.insert("/", "root");
    /// tree.insert("/api", "api");
    /// tree.insert("/api/v2", "api_v2");
    ///
    /// assert_eq!(tree.find_longest_prefix("/api/v2/users"), Some(&"api_v2"));
    /// assert_eq!(tree.find_longest_prefix("/api/v1/users"), Some(&"api"));
    /// assert_eq!(tree.find_longest_prefix("/other"), Some(&"root"));
    /// assert_eq!(tree.find_longest_prefix("nope"), None);
    /// ```
    #[must_use]
    pub fn find_longest_prefix(&self, key: &str) -> Option<&V> {
        self.root.find_longest_prefix(key)
    }

    /// Find all values whose keys are prefixes of the input, longest last.
    ///
    /// Useful when you need to try shorter prefixes if the longest fails.
    #[must_use]
    pub fn find_all_prefixes(&self, key: &str) -> Vec<&V> {
        let mut results = Vec::new();
        self.root.find_all_prefixes(key, &mut results);
        results
    }
}

impl<V> Node<V> {
    fn new(prefix: String) -> Self {
        Self {
            prefix,
            value: None,
            children: HashMap::new(),
        }
    }

    fn insert(&mut self, key: &str, value: V) -> Option<V> {
        // Key exhausted - store value here
        if key.is_empty() {
            return self.value.replace(value);
        }

        let first_byte = key.as_bytes()[0];

        // Check if we have a child with this first character
        if let Some(child) = self.children.get_mut(&first_byte) {
            let common_len = common_prefix_len(key, &child.prefix);

            if common_len == child.prefix.len() {
                // Child prefix fully matches, recurse with remaining key
                return child.insert(&key[common_len..], value);
            }

            // Need to split: create intermediate node
            let mut split_node = Node::new(key[..common_len].to_string());

            // Move child under split node with truncated prefix
            let mut old_child = self.children.remove(&first_byte).unwrap();
            old_child.prefix = old_child.prefix[common_len..].to_string();
            let old_child_first = old_child.prefix.as_bytes()[0];
            split_node.children.insert(old_child_first, old_child);

            // Insert new value
            if common_len == key.len() {
                // Key exactly matches split point
                split_node.value = Some(value);
            } else {
                // Create new leaf for remaining key
                let remaining = &key[common_len..];
                let mut new_leaf = Node::new(remaining.to_string());
                new_leaf.value = Some(value);
                split_node
                    .children
                    .insert(remaining.as_bytes()[0], new_leaf);
            }

            self.children.insert(first_byte, split_node);
            None
        } else {
            // No matching child, create new leaf
            let mut new_node = Node::new(key.to_string());
            new_node.value = Some(value);
            self.children.insert(first_byte, new_node);
            None
        }
    }

    fn get(&self, key: &str) -> Option<&V> {
        if key.is_empty() {
            return self.value.as_ref();
        }

        let first_byte = key.as_bytes()[0];
        let child = self.children.get(&first_byte)?;

        // Check prefix match
        if key.len() >= child.prefix.len() && key.starts_with(&child.prefix) {
            child.get(&key[child.prefix.len()..])
        } else {
            None
        }
    }

    fn find_longest_prefix(&self, key: &str) -> Option<&V> {
        let mut current = self;
        let mut remaining = key;
        let mut last_match: Option<&V> = None;

        // Check root value
        if current.value.is_some() {
            last_match = current.value.as_ref();
        }

        loop {
            if remaining.is_empty() {
                break;
            }

            let first_byte = remaining.as_bytes()[0];
            let Some(child) = current.children.get(&first_byte) else {
                break;
            };

            // Check if child prefix matches
            if remaining.len() >= child.prefix.len() && remaining.starts_with(&child.prefix) {
                remaining = &remaining[child.prefix.len()..];
                current = child;

                if current.value.is_some() {
                    last_match = current.value.as_ref();
                }
            } else {
                break;
            }
        }

        last_match
    }

    fn find_all_prefixes<'a>(&'a self, key: &str, results: &mut Vec<&'a V>) {
        let mut current = self;
        let mut remaining = key;

        // Check root value
        if let Some(ref v) = current.value {
            results.push(v);
        }

        loop {
            if remaining.is_empty() {
                break;
            }

            let first_byte = remaining.as_bytes()[0];
            let Some(child) = current.children.get(&first_byte) else {
                break;
            };

            // Check if child prefix matches
            if remaining.len() >= child.prefix.len() && remaining.starts_with(&child.prefix) {
                remaining = &remaining[child.prefix.len()..];
                current = child;

                if let Some(ref v) = current.value {
                    results.push(v);
                }
            } else {
                break;
            }
        }
    }
}

/// Find the length of the common prefix between two strings.
#[inline]
fn common_prefix_len(a: &str, b: &str) -> usize {
    a.bytes().zip(b.bytes()).take_while(|(x, y)| x == y).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_get() {
        let mut tree = RadixTree::new();
        tree.insert("hello", 1);
        tree.insert("world", 2);
        tree.insert("help", 3);

        assert_eq!(tree.get("hello"), Some(&1));
        assert_eq!(tree.get("world"), Some(&2));
        assert_eq!(tree.get("help"), Some(&3));
        assert_eq!(tree.get("hell"), None);
        assert_eq!(tree.get("helper"), None);
    }

    #[test]
    fn test_prefix_splitting() {
        let mut tree = RadixTree::new();
        tree.insert("test", 1);
        tree.insert("testing", 2);
        tree.insert("team", 3);

        assert_eq!(tree.get("test"), Some(&1));
        assert_eq!(tree.get("testing"), Some(&2));
        assert_eq!(tree.get("team"), Some(&3));
    }

    #[test]
    fn test_find_longest_prefix() {
        let mut tree = RadixTree::new();
        tree.insert("/", "root");
        tree.insert("/api", "api");
        tree.insert("/api/v2", "api_v2");
        tree.insert("/api/v2/users", "users");

        assert_eq!(
            tree.find_longest_prefix("/api/v2/users/123"),
            Some(&"users")
        );
        assert_eq!(tree.find_longest_prefix("/api/v2/posts"), Some(&"api_v2"));
        assert_eq!(tree.find_longest_prefix("/api/v1"), Some(&"api"));
        assert_eq!(tree.find_longest_prefix("/other"), Some(&"root"));
        assert_eq!(tree.find_longest_prefix("/"), Some(&"root"));
        assert_eq!(tree.find_longest_prefix("nope"), None);
    }

    #[test]
    fn test_find_all_prefixes() {
        let mut tree = RadixTree::new();
        tree.insert("/", 1);
        tree.insert("/api", 2);
        tree.insert("/api/v2", 3);

        let prefixes = tree.find_all_prefixes("/api/v2/users");
        assert_eq!(prefixes, vec![&1, &2, &3]);

        let prefixes = tree.find_all_prefixes("/api/v1");
        assert_eq!(prefixes, vec![&1, &2]);
    }

    #[test]
    fn test_overwrite() {
        let mut tree = RadixTree::new();
        assert_eq!(tree.insert("key", 1), None);
        assert_eq!(tree.insert("key", 2), Some(1));
        assert_eq!(tree.get("key"), Some(&2));
    }

    #[test]
    fn test_empty_key() {
        let mut tree = RadixTree::new();
        tree.insert("", "root");
        assert_eq!(tree.get(""), Some(&"root"));
        assert_eq!(tree.find_longest_prefix("anything"), Some(&"root"));
    }
}
