# Writing an item: a guide for authors

This guide is for the C6 test (`docs/authoring-convention.md` section 8): you write one new item yourself, with no
engineering help, and we measure what it cost. You need this page and a text editor, nothing else.

## 1. Start from an example

Copy one of these files and change it:

- `fixtures/items/simple-circuit.item.ron`: physics (a cell, a switch, a lamp);
- `fixtures/items/water-reaction.item.ron`: a chemical reaction;
- `fixtures/items/salt-dissolving.item.ron`: a crystal dissolving.

Save your file in `fixtures/items/`, with a name ending in `.item.ron`.

## 2. What an item holds

```ron
Item(
    id: "my-item",                       // lowercase letters, digits and hyphens
    title: "What the learner sees as the title",
    subject: Physics,                    // Physics, Chemistry or Biology
    idea: "The one idea this item teaches, in one sentence.",
    reference: "The textbook account you are following.",
    objects: [ ... ],                    // what is in the picture (section 3)
    labels: ["..."],                     // names labelled from the start (optional)
    story: [ Beat(...), Beat(...) ],     // what happens, one caption at a time (section 4)
)
```

You never write a position, a size or a time. The kit places everything, and the captions set the pace.

## 3. Objects

**Circuits.**

- `Cell(id: "cell")`.
- `Switch(id: "switch")`. It starts open. Write `starts: Closed` if you want it closed at the start.
- `Lamp(id: "lamp")`.
- `Loop(["cell", "switch", "lamp"])` joins them in one loop, starting at the cell. You can add more switches or lamps.

**Chemistry.**

- `Reaction(id: "reaction", equation: "2 H2 + O2 -> 2 H2O")`. Molecules that can be used: H2, O2, N2, H2O, CO2, CH4 and
  NH3. The equation must balance.
- `Crystal(id: "salt")`: a salt (sodium chloride) crystal.

An item uses either circuit objects or chemistry objects, not both.

## 4. The story

Each `Beat` is one caption and what happens while it is shown:

```ron
Beat(
    say: "The caption. One idea, at most about 45 words, present tense.",
    point_at: ["lamp"],            // highlight and label these, one after another
    label: ["cell.positive"],      // label these at the start of the beat
    unlabel: ["cell.positive"],    // remove these labels
    do: [Close("switch")],         // make something happen
)
```

Only `say` is needed. The beat lasts as long as the caption takes to read (about 2.5 words a second).

| You can `do` | On |
|---|---|
| `Close("switch")`, `Open("switch")` | a switch. The lamp and the current follow by themselves. |
| `React("reaction")` | a reaction |
| `OpenUp("salt")` | a crystal: draws the ions smaller and shows the grid |
| `Dissolve(crystal: "salt", ions: 1)` | a crystal, after `OpenUp` |

You cannot light a lamp or move an atom yourself. The kit does that, so it is always right.

## 5. Names you can point at and label

| Object | Names |
|---|---|
| Cell `cell` | `cell`, `cell.positive`, `cell.negative`, `cell.body` |
| Switch `switch` | `switch`, `switch.lever`, `switch.pivot`, `switch.contact` |
| Lamp `lamp` | `lamp`, `lamp.filament`, `lamp.glass`, `lamp.holder`, `lamp.cap` |
| The loop | `wire`, `wire.1`, `wire.2`, … |
| Reaction `reaction` | `reaction.h2-1`, `reaction.o2-1`, … (the starting molecules); `reaction.h2o-1`, … (the new ones) |
| Crystal `salt` | `salt`, `salt.sodium`, `salt.chloride`, `salt.lines`, `salt.water`, `salt.a-sodium-ion`, `salt.corner-1` … `salt.corner-8` |

If you misspell a name, the error lists every name the item has.

## 6. See it

From the `sci_viz` folder:

```
target\release\svf-explorer.exe --structure fixtures\items\my-item.item.ron --timeline story
```

Space plays and pauses; the arrow keys step through the beats. If the item has a mistake, the message says what and where.

## 7. For the C6 measurement

Please note down:

- how long writing the item took, until it played the way you wanted;
- every error message you met, and whether it told you what to do;
- anything you wanted to say but could not;
- any place you needed help.
