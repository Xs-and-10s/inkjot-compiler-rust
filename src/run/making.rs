//! Things in the hand: taking and dropping, counted items, recipes, and
//! tools that break.
//!
//! ```text
//! default takes: take, drop
//! item stave: "a pit prop" ~ plural "staves" ~ in timber_yard ~ count 4
//! item pick: "a short salt pick" ~ in timber_yard ~ breaks 1 in 6
//! recipe stretcher: stave, stave, sacking ~ make ~ "Two staves and a sack."
//! recipe pawl: horseshoe ~ using hammer ~ make
//! > [[Dig at the fall->marrow:dig]] ~ uses pick
//! ```
//!
//! **Counted items.** `~ count 4` makes four items of one name, `stave#1`
//! to `stave#4`, each an ordinary item: held, dropped and burnt one at a
//! time. The name itself stands for all of them: `count(stave, …)`,
//! `me.holds.stave` (any of them), `lose stave 2`. A list shows them
//! together, "two staves".
//!
//! **Taking and dropping.** `default takes: take, drop` offers a Link to
//! take each item lying here (not `fixed`, not hidden), and to drop each
//! item held: one Link for all of a counted item.
//!
//! **Recipes.** A recipe is offered as a Link, named by its verb (`~ make`:
//! "Make a stretcher"), when the pov holds every part (a part named twice
//! is needed twice) and every tool `using` is held or here, and the thing
//! made is not in the world already. Making uses up the parts, gives the
//! pov the thing, and shows the recipe's text.
//!
//! **Tools.** A Link that `~ uses pick`, and a recipe `using` one, uses the
//! tool when it is taken. A tool that `~ breaks 1 in 6` breaks one time in
//! six: `pick.broken` is set, and the pov is told. Tools do not break
//! unless they say so.

use std::sync::Arc;

use super::{f, strs, Run, R, V};
use crate::json::Value;

impl Run {
    /// The Links `default takes: take, drop` offers here.
    pub(super) fn take_links(&mut self) -> R<Vec<Value>> {
        let story = Arc::clone(&self.story);
        let acts: Vec<String> = match story.defaults.get("takes").and_then(Value::as_str) {
            Some(list) => list.split(',').map(|a| a.trim().to_string()).collect(),
            None => return Ok(Vec::new()),
        };
        let pov = self.pov.clone();
        let here = self.location(&pov);
        let mut links = Vec::new();
        let mut seen = Vec::new();
        for (item, holder) in self.holder.clone() {
            let lying = holder == here;
            let held = holder == pov;
            if !lying && !held {
                continue;
            }
            let act = if lying { "take" } else { "drop" };
            if !acts.iter().any(|a| a == act) {
                continue;
            }
            if lying && (self.tagged(&item, "fixed") || !self.shown(&item)?) {
                continue;
            }
            // One Link for all of a counted item: the first of them.
            let name = story.of(&item).unwrap_or(&item).to_string();
            if seen.contains(&(act, name.clone())) {
                continue;
            }
            seen.push((act, name.clone()));
            let verb = if lying { "Take" } else { "Drop" };
            links.push(Value::obj([
                ("type", Value::str("link")),
                ("id", Value::str(format!("{act}:{name}"))),
                ("label", Value::str(format!("{verb} {}", self.label(&item)))),
                ("target", Value::str("here")),
                ("kind", Value::str("act")),
                ("act", Value::str(act)),
                ("arg", Value::str(&item)),
            ]));
        }
        Ok(links)
    }

    /// Is an item to be seen: not `hidden until` something that has not
    /// happened yet?
    fn shown(&mut self, item: &str) -> R<bool> {
        let until = self.story.field(item, "hidden_until").clone();
        Ok(until == Value::Null || self.eval(&until)?.truthy())
    }

    /// Drop an item where the pov stands.
    pub(super) fn drop_item(&mut self, item: &str) {
        let here = self.location(&self.pov.clone());
        self.holder.insert(item.to_string(), here);
    }

    /// The Links the recipes offer: each one the pov could make now.
    pub(super) fn make_links(&mut self) -> R<Vec<Value>> {
        let mut links = Vec::new();
        for recipe in self.story.recipes.clone() {
            let made = f(&recipe, "recipe").as_str().unwrap_or("").to_string();
            if self.holder.contains_key(&made) || self.parts_held(&recipe).is_none() {
                continue;
            }
            let tools = strs(f(&recipe, "using"));
            if !tools.iter().all(|t| self.at_hand(t)) {
                continue;
            }
            let verb = f(&recipe, "verb").as_str().unwrap_or("make");
            links.push(Value::obj([
                ("type", Value::str("link")),
                ("id", Value::str(format!("{verb}:{made}"))),
                (
                    "label",
                    Value::str(format!("{} {}", capital(verb), self.label(&made))),
                ),
                ("target", Value::str("here")),
                ("kind", Value::str("make")),
                ("arg", Value::str(&made)),
                ("uses", f(&recipe, "using").clone()),
            ]));
        }
        Ok(links)
    }

    /// The items the pov would use up for a recipe, or `None` if they do
    /// not hold every part.
    fn parts_held(&self, recipe: &Value) -> Option<Vec<String>> {
        let mut held: Vec<String> = self
            .holder
            .iter()
            .filter(|(_, h)| **h == self.pov)
            .map(|(i, _)| i.clone())
            .collect();
        held.sort();
        let mut used = Vec::new();
        for part in strs(f(recipe, "parts")) {
            let i = held
                .iter()
                .position(|h| *h == part || self.story.of(h) == Some(part.as_str()))?;
            used.push(held.remove(i));
        }
        Some(used)
    }

    /// Is a tool held by the pov, or lying here?
    fn at_hand(&self, tool: &str) -> bool {
        let here = self.location(&self.pov);
        self.holder
            .get(tool)
            .is_some_and(|h| *h == self.pov || *h == here)
    }

    /// Make what a recipe makes: use up the parts, hold the thing, and say
    /// what it is.
    pub(super) fn make(&mut self, made: &str) -> R<()> {
        let recipe = self
            .story
            .recipes
            .iter()
            .find(|r| f(r, "recipe").as_str() == Some(made))
            .cloned()
            .ok_or_else(|| format!("there is no recipe for `{made}`"))?;
        let parts = self
            .parts_held(&recipe)
            .ok_or_else(|| format!("the parts for `{made}` are not all held"))?;
        for part in parts {
            self.destroy_item(&part);
        }
        let pov = self.pov.clone();
        self.vals.remove(&format!("{made}.destroyed"));
        self.hold(made, &pov);
        if let Some(text) = f(&recipe, "label").as_str() {
            self.output.push(text.into());
        } else if let text @ Value::Obj(_) = f(&recipe, "label") {
            let text = self.text_of(&text.clone())?;
            self.output.push(text.as_str().into());
        }
        Ok(())
    }

    /// An item is gone from the world: used up, burnt, broken up.
    pub(super) fn destroy_item(&mut self, item: &str) {
        self.holder.remove(item);
        self.vals.insert(format!("{item}.destroyed"), V::Bool(true));
    }

    /// The tools a Link `~ uses`, each of which may break.
    pub(super) fn use_tools(&mut self, link: &Value) -> R<bool> {
        let mut broke = false;
        for tool in strs(f(link, "uses")) {
            let breaks = self.story.field(&tool, "breaks").clone();
            let (Some(chance), Some(sides)) =
                (f(&breaks, "chance").as_u64(), f(&breaks, "in").as_u64())
            else {
                continue;
            };
            if self.roll(sides) <= chance {
                self.vals.insert(format!("{tool}.broken"), V::Bool(true));
                let text = format!("{} breaks.", capital(&definite(&self.label(&tool))));
                self.output.push(text.as_str().into());
                broke = true;
            }
        }
        Ok(broke)
    }

    /// The items an effect's argument names: an item, a counted item's
    /// name, a `let`, or an expression for one or many (`all(item, …)`).
    pub(super) fn items_named(&mut self, arg: &Value) -> R<Vec<String>> {
        if let Some(name) = arg.as_str() {
            if self.story.kind(name).is_some() && !self.locals.contains_key(name) {
                return Ok(vec![name.to_string()]);
            }
        }
        Ok(match self.eval(arg)? {
            V::Id(id) | V::Str(id) => vec![id],
            V::List(items) => items
                .iter()
                .filter_map(|i| i.name().map(str::to_string))
                .collect(),
            _ => Vec::new(),
        })
    }

    /// `lose stave 2`: the first `n` of a counted item the pov holds.
    pub(super) fn held_of(&self, name: &str, n: usize) -> Vec<String> {
        let mut held: Vec<String> = self
            .holder
            .iter()
            .filter(|(i, h)| **h == self.pov && self.story.of(i) == Some(name))
            .map(|(i, _)| i.clone())
            .collect();
        held.sort();
        held.truncate(n);
        held
    }

    /// `gain stave 2`: the first `n` of a counted item that are nowhere
    /// yet, and not used up.
    pub(super) fn spare_of(&self, name: &str, n: usize) -> Vec<String> {
        let mut spare: Vec<String> = self
            .story
            .decls
            .keys()
            .filter(|i| self.story.of(i) == Some(name) && !self.holder.contains_key(*i))
            .cloned()
            .collect();
        spare.sort();
        spare.truncate(n);
        spare
    }
}

impl Run {
    /// Several of a counted item, as a list shows them: "two staves", with
    /// its `plural`, or its label made plural.
    pub(super) fn several(&self, name: &str, n: usize) -> String {
        let noun = match self.story.field(name, "plural").as_str() {
            Some(plural) => plural.to_string(),
            None => {
                let one = self.label(name);
                let one = ["a ", "an ", "some "]
                    .iter()
                    .find_map(|a| one.strip_prefix(a))
                    .unwrap_or(&one)
                    .to_string();
                super::plural_of(&one)
            }
        };
        format!("{} {noun}", number_word(n))
    }

    /// `table rank: score ~ 0..=19: "a lamp carrier"`: the value of the
    /// first row the table's value falls in. `None` for a table whose rows
    /// are effects, not values.
    pub(super) fn table_value(&mut self, table: &str) -> R<Option<V>> {
        let rows = self
            .story
            .field(table, "rows")
            .as_arr()
            .unwrap_or(&[])
            .to_vec();
        if !rows.iter().any(|r| r.get("value").is_some()) {
            return Ok(None);
        }
        let read = self.story.field(table, "roll").clone();
        let v = self.eval(&read)?;
        for row in rows {
            if !v.within(&self.constant(f(&row, "range"))) {
                continue;
            }
            return Ok(Some(match f(&row, "value") {
                Value::Str(text) => V::Str(text.clone()),
                text @ Value::Obj(_) if text.get("text").is_some() => {
                    V::Str(self.text_of(&text.clone())?)
                }
                Value::Null => V::Null,
                e => self.eval(&e.clone())?,
            }));
        }
        Ok(Some(V::Null))
    }
}

/// Two to twelve in words, and figures after.
fn number_word(n: usize) -> String {
    const WORDS: [&str; 13] = [
        "no", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
        "eleven", "twelve",
    ];
    WORDS
        .get(n)
        .map_or_else(|| n.to_string(), |w| w.to_string())
}

/// `make` → `Make`.
fn capital(s: &str) -> String {
    let mut chars = s.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

/// `a short salt pick` → `the short salt pick`.
pub(super) fn definite(label: &str) -> String {
    for article in ["a ", "an ", "some "] {
        if let Some(rest) = label.strip_prefix(article) {
            return format!("the {rest}");
        }
    }
    label.to_string()
}
