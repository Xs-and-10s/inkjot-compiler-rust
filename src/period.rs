//! Periods: named stretches of the clock (Inkjot v0.6, decided 21 Nov 2026).
//!
//! ```text
//! period watch: morning 1, afternoon 2, evening 1, night 1
//! period day: monday, tuesday, wednesday ~ of watch
//! clock: monday.morning ..= wednesday.night
//! ```
//!
//! A period lists its names. Each name lasts a number of turns: its own
//! (`afternoon 2`), the same for all (`spring, summer ~ 3 each`), or one
//! round of another period (`~ of watch`: a day is morning to night). A
//! time is a path from the outermost period inward: `monday.evening` is the
//! evening of Monday, `monday` the whole of it.
//!
//! This module is the arithmetic only: turning names into turns and back.
//! The checker and the interpreter will use it when periods are built; the
//! draft examples' tests use it now, to check every `@ monday.evening`
//! they write.

use std::ops::Range;

/// One `period` statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Period {
    pub name: String,
    pub names: Vec<String>,
    pub each: Each,
}

/// How long each name of a period lasts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Each {
    /// `morning 1, afternoon 2`: one length per name, in turns.
    Own(Vec<u32>),
    /// `~ 3 each`: the same number of turns for every name.
    Turns(u32),
    /// `~ of watch`: one round of another period.
    Of(String),
}

/// Every period of an Inklink.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Calendar {
    pub periods: Vec<Period>,
}

impl Period {
    /// Read the text after `period `: `day: monday, tuesday ~ of watch`.
    pub fn parse(text: &str) -> Result<Period, String> {
        let text = text.trim().trim_start_matches("period").trim();
        let (name, rest) = text
            .split_once(':')
            .ok_or_else(|| format!("`period {text}` needs its names after a colon"))?;
        let name = name.trim().to_string();
        let mut clauses = rest.split('~').map(str::trim);
        let list = clauses.next().unwrap_or("");
        let mut names = Vec::new();
        let mut own = Vec::new();
        for item in list.split(',').map(str::trim).filter(|i| !i.is_empty()) {
            let mut words = item.split_whitespace();
            let n = words.next().unwrap_or("").to_string();
            match words.next() {
                Some(len) => {
                    own.push(len.parse::<u32>().ok().filter(|l| *l > 0).ok_or_else(|| {
                        format!("`{item}`: a length is a whole number of turns, 1 or more")
                    })?)
                }
                None if !own.is_empty() => {
                    return Err(format!("`{n}` needs a length, like the names before it"))
                }
                None => {}
            }
            names.push(n);
        }
        if names.is_empty() {
            return Err(format!("`period {name}` names nothing"));
        }
        let mut each = None;
        for clause in clauses {
            if let Some(unit) = clause.strip_prefix("of ") {
                each = Some(Each::Of(unit.trim().to_string()));
            } else if let Some(n) = clause.strip_suffix(" each") {
                let n = n
                    .trim()
                    .parse::<u32>()
                    .ok()
                    .filter(|l| *l > 0)
                    .ok_or_else(|| format!("`~ {clause}`: a length is a whole number of turns"))?;
                each = Some(Each::Turns(n));
            }
        }
        let each = match (each, own.is_empty()) {
            (Some(_), false) => {
                return Err(format!(
                    "`period {name}` gives lengths twice: after each name and in a clause"
                ))
            }
            (Some(e), true) => e,
            (None, false) if own.len() == names.len() => Each::Own(own),
            (None, false) => return Err(format!("every name of `period {name}` needs a length")),
            (None, true) => Each::Turns(1),
        };
        Ok(Period { name, names, each })
    }
}

impl Calendar {
    /// Read every `period` statement among `lines`; other lines are skipped.
    pub fn parse<'a>(lines: impl IntoIterator<Item = &'a str>) -> Result<Calendar, String> {
        let mut periods: Vec<Period> = Vec::new();
        for line in lines {
            let t = line.trim();
            if t.starts_with("period ") {
                let p = Period::parse(t)?;
                if periods.iter().any(|q| q.name == p.name) {
                    return Err(format!("`period {}` is declared twice", p.name));
                }
                periods.push(p);
            }
        }
        Calendar::new(periods)
    }

    /// A calendar of these periods, if they make one: every `of` names a
    /// period, and none is made of itself.
    pub fn new(periods: Vec<Period>) -> Result<Calendar, String> {
        let cal = Calendar { periods };
        for p in &cal.periods {
            if let Each::Of(unit) = &p.each {
                if cal.get(unit).is_none() {
                    return Err(format!(
                        "`period {}` is made `of {unit}`, which is not a period",
                        p.name
                    ));
                }
            }
            // A cycle (`day ~ of week`, `week ~ of day`) has no length.
            if cal.round(&p.name).is_none() {
                return Err(format!("`period {}` is made of itself", p.name));
            }
        }
        Ok(cal)
    }

    pub fn get(&self, name: &str) -> Option<&Period> {
        self.periods.iter().find(|p| p.name == name)
    }

    /// Turns in one round of a period: every name, once.
    pub fn round(&self, period: &str) -> Option<u32> {
        self.round_within(period, self.periods.len())
    }

    fn round_within(&self, period: &str, depth: usize) -> Option<u32> {
        let p = self.get(period)?;
        let each = |i: usize| -> Option<u32> {
            match &p.each {
                Each::Own(lens) => lens.get(i).copied(),
                Each::Turns(n) => Some(*n),
                Each::Of(unit) => depth
                    .checked_sub(1)
                    .and_then(|d| self.round_within(unit, d)),
            }
        };
        (0..p.names.len()).map(each).sum()
    }

    /// How many turns one name of a period lasts.
    pub fn length(&self, period: &str, name: &str) -> Option<u32> {
        let p = self.get(period)?;
        let i = p.names.iter().position(|n| n == name)?;
        match &p.each {
            Each::Own(lens) => lens.get(i).copied(),
            Each::Turns(n) => Some(*n),
            Each::Of(unit) => self.round(unit),
        }
    }

    /// The outermost period: the one no other period is made of. With
    /// several unrelated periods (seasons, and the hours of a day), the
    /// longest round wins.
    pub fn outermost(&self) -> Option<&Period> {
        let inner: Vec<&str> = self
            .periods
            .iter()
            .filter_map(|p| match &p.each {
                Each::Of(u) => Some(u.as_str()),
                _ => None,
            })
            .collect();
        self.periods
            .iter()
            .filter(|p| !inner.contains(&p.name.as_str()))
            .max_by_key(|p| self.round(&p.name).unwrap_or(0))
    }

    /// The turns a time covers, from the start of the outermost period:
    /// `monday.evening`, or `monday` for all of it.
    pub fn span(&self, path: &str) -> Option<Range<u32>> {
        let mut period = self.outermost()?;
        let mut start = 0;
        let mut segments = path.split('.').peekable();
        while let Some(seg) = segments.next() {
            let i = period.names.iter().position(|n| n == seg)?;
            start += (0..i)
                .map(|j| self.length(&period.name, &period.names[j]))
                .sum::<Option<u32>>()?;
            let len = self.length(&period.name, seg)?;
            if segments.peek().is_none() {
                return Some(start..start + len);
            }
            match &period.each {
                Each::Of(unit) => period = self.get(unit)?,
                _ => return None,
            }
        }
        None
    }

    /// The names a turn falls in, outermost first: turn 7 of a week of
    /// five-turn days is `[("day", "tuesday"), ("watch", "afternoon")]`.
    /// A clock longer than one round goes round again.
    pub fn at(&self, turn: u32) -> Vec<(String, String)> {
        self.walk(turn)
            .into_iter()
            .map(|(p, n, _)| (p, n))
            .collect()
    }

    /// The names that begin on a turn, outermost first: on turn 5 of a
    /// week of five-turn days, `[("day", "tuesday"), ("watch", "morning")]`;
    /// on turn 6, the afternoon only. Turn 0 begins everything.
    pub fn begins(&self, turn: u32) -> Vec<(String, String)> {
        self.walk(turn)
            .into_iter()
            .filter(|(_, _, offset)| *offset == 0)
            .map(|(p, n, _)| (p, n))
            .collect()
    }

    /// Each period a turn falls in, outermost first, with the name and how
    /// many turns into it the turn is.
    fn walk(&self, turn: u32) -> Vec<(String, String, u32)> {
        let mut out = Vec::new();
        let Some(mut period) = self.outermost() else {
            return out;
        };
        let Some(round) = self.round(&period.name).filter(|r| *r > 0) else {
            return out;
        };
        let mut t = turn % round;
        loop {
            let mut found = None;
            for name in &period.names {
                let len = self.length(&period.name, name).unwrap_or(0);
                if t < len {
                    found = Some(name.clone());
                    break;
                }
                t -= len;
            }
            let Some(name) = found else {
                return out;
            };
            out.push((period.name.clone(), name, t));
            match &period.each {
                Each::Of(unit) => match self.get(unit) {
                    Some(p) => period = p,
                    None => return out,
                },
                _ => return out,
            }
        }
    }

    /// The period a name belongs to: `night` is the `phase`'s.
    pub fn period_of(&self, name: &str) -> Option<&Period> {
        self.periods
            .iter()
            .find(|p| p.names.iter().any(|n| n == name))
    }

    /// Where a name comes in its period, from 0: `day` is 1 in `night, day`.
    pub fn index(&self, name: &str) -> Option<usize> {
        self.period_of(name)?.names.iter().position(|n| n == name)
    }

    /// Is `time` a time of this calendar: a name (`night`), or a path of
    /// names, each inside the one before (`new_years_eve.nine`)?
    pub fn is_time(&self, time: &str) -> bool {
        let mut names = time.split('.');
        let Some(mut period) = names.next().and_then(|n| self.period_of(n)) else {
            return false;
        };
        for name in names {
            let Each::Of(unit) = &period.each else {
                return false;
            };
            match self.get(unit) {
                Some(p) if p.names.iter().any(|n| n == name) => period = p,
                _ => return false,
            }
        }
        true
    }

    /// Does `time` begin on this turn? `night` when a night does;
    /// `new_years_eve.nine` when nine does, on New Year's Eve.
    pub fn begins_on(&self, time: &str, turn: u32) -> bool {
        let path: Vec<&str> = time.split('.').collect();
        let Some(first) = self.period_of(path[0]) else {
            return false;
        };
        let levels = self.walk(turn);
        let Some(i) = levels.iter().position(|(p, _, _)| *p == first.name) else {
            return false;
        };
        let inside = &levels[i..];
        inside.len() >= path.len()
            && path.iter().zip(inside).all(|(n, (_, m, _))| n == m)
            && inside[path.len() - 1].2 == 0
    }

    /// The first turn, at or after `from`, on which `time` begins, within
    /// one round of the outermost period; `None` if it never does.
    pub fn next_start(&self, time: &str, from: u32) -> Option<u32> {
        let round = self.round(&self.outermost()?.name)?;
        (from..=from + round).find(|t| self.begins_on(time, *t))
    }

    /// `monday.morning ..= saturday.night`: the turns a clock runs over.
    pub fn clock(&self, range: &str) -> Option<Range<u32>> {
        let (from, to) = range.split_once("..=")?;
        let a = self.span(from.trim())?;
        let b = self.span(to.trim())?;
        (a.start < b.end).then_some(a.start..b.end)
    }
}
