//! Ex 7's rules, played out. The draft can't run yet (decks, tables,
//! periods and group choices aren't built), so this is a model of its rules
//! (the deck's sizes, who may be chosen at night, debts, conversions,
//! thefts, a vote whose ties are drawn) played by players who choose at
//! random. The draft's own numbers are checked first, so the model can't
//! drift from them unnoticed.
//!
//! What it shows: every game ends, whatever anyone does; and for every
//! syndicate at every table size, both sides can win.

use proptest::prelude::*;

fn draft() -> String {
    let dir = env!("CARGO_MANIFEST_DIR");
    std::fs::read_to_string(format!("{dir}/examples/drafts/ex7-mafia.twee")).unwrap()
}

/// The lines of the draft this model copies.
#[test]
fn the_model_follows_the_draft() {
    let src = draft();
    for line in [
        "get members: max(1, players / 4)",
        "~ card syndicate: members",
        "~ card doctor: 1",
        "~ card detective: 1",
        "~ card townsfolk: rest",
        "get haul_needed: members + 2",
        "revote: 1, ties: draw}",
        "it.marked_since < clock.now)",
        "set victim.marked_since: clock.now when victim != nobody",
        "let foiled: victim != nobody and victim == saved",
        "set victim.team: cult when victim != nobody",
        "it != me.saved_last",
        "it.voted_with(cult)",
        "finale when: syndicate_left >= town_left and syndicate.special in [none, doppelganger, collectors]",
        "finale when: syndicate.special == cult and cult_left > town_left",
        "finale when: syndicate.special == heist and count(haul) >= haul_needed",
    ] {
        assert!(src.contains(line), "the draft no longer says `{line}`");
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Special {
    None,
    Doppelganger,
    Collectors,
    Cult,
    Heist,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Team {
    Town,
    Syndicate,
    Cult,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Role {
    Syndicate,
    Doctor,
    Detective,
    Townsfolk,
}

#[derive(Clone, Debug)]
struct Guest {
    alive: bool,
    team: Team,
    role: Role,
    marked_since: Option<u32>,
    voteless: bool,
    keepsake: bool,
}

#[derive(Debug, PartialEq)]
enum Winner {
    Town,
    Syndicate,
    Cult,
    Heist,
}

/// xorshift, as the engine's dice are.
struct Dice(u64);
impl Dice {
    fn pick(&mut self, n: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n as u64) as usize
    }
    fn one(&mut self, from: &[usize]) -> Option<usize> {
        (!from.is_empty()).then(|| from[self.pick(from.len())])
    }
}

fn members(players: usize) -> usize {
    (players / 4).max(1)
}

fn deal(players: usize, special: Special, dice: &mut Dice) -> Vec<Guest> {
    let m = members(players);
    let mut deck: Vec<Role> = std::iter::repeat_n(Role::Syndicate, m)
        .chain([Role::Doctor, Role::Detective])
        .chain(std::iter::repeat_n(Role::Townsfolk, players - m - 2))
        .collect();
    for i in (1..deck.len()).rev() {
        deck.swap(i, dice.pick(i + 1));
    }
    deck.into_iter()
        .map(|role| Guest {
            alive: true,
            team: match (role, special) {
                (Role::Syndicate, Special::Cult) => Team::Cult,
                (Role::Syndicate, _) => Team::Syndicate,
                _ => Team::Town,
            },
            role,
            marked_since: None,
            voteless: false,
            keepsake: true,
        })
        .collect()
}

fn alive(g: &[Guest], f: impl Fn(&Guest) -> bool) -> Vec<usize> {
    (0..g.len()).filter(|&i| g[i].alive && f(&g[i])).collect()
}

/// One game, to the end. `None` if it is still going after `limit` phases.
fn play(players: usize, special: Special, seed: u64, limit: u32) -> Option<Winner> {
    let mut dice = Dice(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let mut g = deal(players, special, &mut dice);
    let need = members(players) + 2;
    let mut haul = 0;
    let mut saved_last: Option<usize> = None;
    // Who voted with a cultist yesterday.
    let mut voted_with_cult: Vec<usize> = Vec::new();
    for now in 0..limit {
        // `finale when:`, in the draft's order.
        let town = alive(&g, |x| x.team == Team::Town).len();
        let synd = alive(&g, |x| x.team == Team::Syndicate).len();
        let cult = alive(&g, |x| x.team == Team::Cult).len();
        if alive(&g, |x| x.team != Team::Town).is_empty() {
            return Some(Winner::Town);
        }
        if matches!(
            special,
            Special::None | Special::Doppelganger | Special::Collectors
        ) && synd >= town
        {
            return Some(Winner::Syndicate);
        }
        if special == Special::Cult && cult > town {
            return Some(Winner::Cult);
        }
        if special == Special::Heist && haul >= need {
            return Some(Winner::Heist);
        }
        if now % 2 == 0 {
            // Night. Debts fall due first.
            if special == Special::Collectors {
                for x in g.iter_mut() {
                    if x.alive && x.marked_since.is_some_and(|t| t < now) {
                        x.alive = false;
                    }
                }
            }
            let not_ours = |x: &Guest| x.team == Team::Town;
            let target = match special {
                Special::Cult => {
                    let can = alive(&g, not_ours);
                    dice.one(
                        &can.into_iter()
                            .filter(|i| voted_with_cult.contains(i))
                            .collect::<Vec<_>>(),
                    )
                }
                Special::Collectors => {
                    dice.one(&alive(&g, |x| not_ours(x) && x.marked_since.is_none()))
                }
                Special::Heist => dice.one(&alive(&g, |x| not_ours(x) && x.keepsake)),
                _ => dice.one(&alive(&g, not_ours)),
            };
            let doctor_alive = !alive(&g, |x| x.role == Role::Doctor).is_empty();
            let saved = if doctor_alive {
                let can: Vec<usize> = alive(&g, |_| true)
                    .into_iter()
                    .filter(|i| Some(*i) != saved_last)
                    .collect();
                dice.one(&can)
            } else {
                None
            };
            saved_last = saved;
            // The morning settles the night: a save stops every special's
            // choice. A mark is made in the morning, `now + 1`.
            if let Some(t) = target.filter(|t| saved != Some(*t)) {
                match special {
                    Special::None | Special::Doppelganger => g[t].alive = false,
                    Special::Collectors => g[t].marked_since = Some(now + 1),
                    Special::Cult => g[t].team = Team::Cult,
                    Special::Heist => {
                        g[t].keepsake = false;
                        haul += 1;
                    }
                }
            }
        } else {
            // Day. Someone may pay a debt, and loses their vote.
            if special == Special::Collectors && dice.pick(2) == 0 {
                let marked = alive(&g, |x| x.marked_since.is_some());
                let payers = alive(&g, |x| !x.voteless);
                if let (Some(m), Some(p)) = (dice.one(&marked), dice.one(&payers)) {
                    g[m].marked_since = None;
                    g[p].voteless = true;
                }
            }
            let voters = alive(&g, |x| !x.voteless);
            let candidates = alive(&g, |_| true);
            let mut votes = vec![0usize; g.len()];
            let mut ballots = Vec::new();
            for &v in &voters {
                let choice = candidates[dice.pick(candidates.len())];
                votes[choice] += 1;
                ballots.push((v, choice));
            }
            let top = *votes.iter().max().unwrap();
            let tied: Vec<usize> = (0..g.len())
                .filter(|&i| votes[i] == top && top > 0)
                .collect();
            // A tie votes once more; modelled as straight to the draw.
            if let Some(out) = dice.one(&tied) {
                g[out].alive = false;
            }
            let cult_choices: Vec<usize> = ballots
                .iter()
                .filter(|(v, _)| g[*v].team == Team::Cult)
                .map(|(_, c)| *c)
                .collect();
            voted_with_cult = ballots
                .iter()
                .filter(|(v, c)| g[*v].team != Team::Cult && cult_choices.contains(c))
                .map(|(v, _)| *v)
                .collect();
            for x in g.iter_mut() {
                x.voteless = false;
            }
        }
    }
    None
}

const SPECIALS: [Special; 5] = [
    Special::None,
    Special::Doppelganger,
    Special::Collectors,
    Special::Cult,
    Special::Heist,
];

#[test]
fn every_deck_deals_a_card_each_with_some_on_both_sides() {
    for players in 5..=12 {
        let mut dice = Dice(7);
        let g = deal(players, Special::None, &mut dice);
        assert_eq!(g.len(), players);
        let m = g.iter().filter(|x| x.role == Role::Syndicate).count();
        assert!(m >= 1 && m == members(players));
        assert!(g.iter().filter(|x| x.role == Role::Townsfolk).count() >= 1);
    }
}

/// For every syndicate, at every table size, the town can win and so can
/// the other side.
#[test]
fn both_sides_can_win_everywhere() {
    for special in SPECIALS {
        for players in 5..=12 {
            let mut town = 0;
            let mut other = 0;
            for seed in 1..=600u64 {
                match play(players, special, seed, 200) {
                    Some(Winner::Town) => town += 1,
                    Some(_) => other += 1,
                    None => {}
                }
            }
            assert!(town > 0, "{special:?} at {players}: the town never wins");
            assert!(other > 0, "{special:?} at {players}: the town always wins");
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(400))]

    /// Every game ends: someone leaves the table every day, so within two
    /// phases a player, whatever anyone does.
    #[test]
    fn every_game_ends(players in 5usize..=12, which in 0usize..5, seed in any::<u64>()) {
        let special = SPECIALS[which];
        let limit = 2 * players as u32 + 2;
        prop_assert!(play(players, special, seed, limit).is_some(), "{:?} at {} did not end", special, players);
    }
}
