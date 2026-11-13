//! `balance_report`: walks the tech tree and prints the raw materials needed
//! per era, so pacing can be checked on paper.
//!
//! Run with `cargo run -p deepcore_shared --bin balance_report`.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use deepcore_shared::content::{ContentRegistry, ItemId, RecipeId};

fn main() {
    let dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "assets/data".to_string());
    let reg = match ContentRegistry::load_dir(Path::new(&dir)) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("failed to load content from {dir}: {e}");
            std::process::exit(1);
        }
    };

    println!("=== Deepcore balance report ===");
    println!(
        "blocks {}  items {}  recipes {}  machines {}  tech nodes {}  layers {}",
        reg.blocks().len(),
        reg.items().len(),
        reg.recipes().len(),
        reg.machines().len(),
        reg.tech_nodes().len(),
        reg.layers().len()
    );
    println!("content hash: {:016x}", reg.hash);

    // Which tech node unlocks each recipe.
    let mut recipe_tech: HashMap<RecipeId, String> = HashMap::new();
    for t in reg.tech_nodes() {
        for u in &t.unlocks {
            if let Some(r) = reg.recipe(u) {
                recipe_tech.insert(r, t.id.clone());
            }
        }
    }
    for r in reg.recipes() {
        if let Some(t) = &r.tech {
            recipe_tech
                .entry(deepcore_shared::content::RecipeId(
                    reg.recipes().iter().position(|x| x.id == r.id).unwrap_or(0) as u16,
                ))
                .or_insert_with(|| t.clone());
        }
    }

    // Expand a bill of materials into raw materials.
    fn expand(
        reg: &ContentRegistry,
        want: &[(ItemId, u32)],
        recipe_for: &HashMap<ItemId, RecipeId>,
        depth: u32,
    ) -> BTreeMap<String, u64> {
        let mut out: BTreeMap<String, u64> = BTreeMap::new();
        if depth > 24 {
            return out;
        }
        for (item, n) in want {
            if is_raw_global(reg, *item) || recipe_for.get(item).is_none() {
                *out.entry(reg.item_str(*item).to_string()).or_default() += *n as u64;
                continue;
            }
            let rid = recipe_for[item];
            let Some(r) = reg.recipe_def(rid) else {
                *out.entry(reg.item_str(*item).to_string()).or_default() += *n as u64;
                continue;
            };
            let per_craft = r
                .outputs
                .iter()
                .filter_map(|(i, c)| reg.item(i).map(|id| (id, *c)))
                .find(|(id, _)| id == item)
                .map(|(_, c)| c)
                .unwrap_or(1)
                .max(1);
            let crafts = (*n).div_ceil(per_craft);
            let inputs: Vec<(ItemId, u32)> = r
                .inputs
                .iter()
                .filter_map(|(i, c)| reg.item(i).map(|id| (id, *c * crafts)))
                .collect();
            for (k, v) in expand(reg, &inputs, recipe_for, depth + 1) {
                *out.entry(k).or_default() += v;
            }
        }
        out
    }

    fn is_raw_global(reg: &ContentRegistry, i: ItemId) -> bool {
        let dropped = reg
            .blocks()
            .iter()
            .any(|b| b.drops.iter().any(|(d, _)| reg.item(d) == Some(i)));
        let produced = reg
            .recipes()
            .iter()
            .any(|r| r.outputs.iter().any(|(o, _)| reg.item(o) == Some(i)));
        dropped && !produced
    }

    // First recipe that produces each item (hand recipes preferred).
    let mut recipe_for: HashMap<ItemId, RecipeId> = HashMap::new();
    for (i, r) in reg.recipes().iter().enumerate() {
        for (out, _) in &r.outputs {
            if let Some(id) = reg.item(out) {
                recipe_for.entry(id).or_insert(RecipeId(i as u16));
            }
        }
    }

    for era in 0..=5u8 {
        let nodes: Vec<_> = reg.tech_nodes().iter().filter(|t| t.era == era).collect();
        if nodes.is_empty() {
            continue;
        }
        let mut total: BTreeMap<String, u64> = BTreeMap::new();
        for node in &nodes {
            let cost: Vec<(ItemId, u32)> = node
                .cost
                .iter()
                .filter_map(|(i, c)| reg.item(i).map(|id| (id, *c)))
                .collect();
            for (k, v) in expand(&reg, &cost, &recipe_for, 0) {
                *total.entry(k).or_default() += v;
            }
        }
        let sum: u64 = total.values().sum();
        println!(
            "\n--- Era {era} ---  ({} nodes, {sum} raw items)",
            nodes.len()
        );
        for (k, v) in &total {
            println!("  {k:<24} {v:>6}");
        }
    }

    // Which recipes are unlocked by nothing.
    let unlocked: HashSet<&str> = reg
        .tech_nodes()
        .iter()
        .flat_map(|t| t.unlocks.iter().map(|s| s.as_str()))
        .collect();
    let orphans: Vec<&str> = reg
        .recipes()
        .iter()
        .filter(|r| !r.hand && !unlocked.contains(r.id.as_str()) && r.tech.is_none())
        .map(|r| r.id.as_str())
        .collect();
    println!("\nrecipes with no tech gate: {}", orphans.len());
    for o in orphans {
        println!("  {o}");
    }
}
