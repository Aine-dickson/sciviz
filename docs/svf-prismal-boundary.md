# SVF and Prismal: which tool carries what

Last updated: 2026-11-22. Status: **working rule from the user, not a spec.**
Integration of SVF, Prismal and Mava is deliberately deferred until each tool's
capabilities are settled (user, 2026-11-22). This note only decides which tool
an item or lesson step should be explored in.

## What Prismal is (read from its repo, 2026-11-22)

A modelling language. An author writes a `model` (parameters with units, state,
`flow` equations, events), a `presentation` (plots, sliders, buttons, figures)
and optional `run` cases that check the result. The learner changes inputs and
watches the outcome, usually as a graph, counts or a moving figure. Its
examples are a projectile, a chain, heat in a spoon, and few versus many
molecules in a reaction. It has 3D views of simple bodies, but not anatomical
meshes, sections or peeling. The author needs to understand rules and variables.

## What SVF is

A viewer of *what something is and how its parts relate*: closed parts with
names and parents, sectioning, peeling, picking and views, built from authored
recipes, adapted source meshes, molecules or fields. The author describes the
picture, not rules.

## The working rule

Ask three questions about a lesson step:

1. **Authoring.** Would the author have to state rules and variables (Prismal),
   or describe the thing or a fixed sequence of steps (SVF)?
2. **Learner.** Does the learner *change inputs and compare outcomes* (Prismal),
   or *inspect and watch one correct account* (SVF)?
3. **Output.** Is the useful result a graph, number or count (Prismal), or a
   spatial structure or scene (SVF)?

Mostly Prismal answers: Prismal. Mostly SVF answers: SVF. A split answer means
both tools, each carrying its half.

| Topic | Tool | Reason |
|---|---|---|
| Rate of plant growth under different factors | Prismal | variables, compared outcomes, a graph |
| Stages of plant growth, seed to fruit | SVF | one correct account, a structure at each stage |
| How water moves from roots to leaves for starch making | SVF | a fixed process shown on a structure; the learner does not vary it |
| How fast water moves under different humidity | Prismal | a variable and a graph |
| H and O combining to make H₂O | SVF | one correct account, shape matters |
| Reaction rate with few or many molecules | Prismal | counts, randomness, a graph (it has this example) |
| Heart structure, sections, peeling | SVF | structure |
| Heart rate against exercise | Prismal | variable and graph |
| Flower structure and stages | SVF | structure (this note's origin) |
| Pollen grains chosen by the learner, seeds that form | Prismal | learner variable and random outcomes |

## Applying it to the 44 written Prismal requirements

From the titles and the texts I read (RQ-016, 021, 030, 031, 034, 035, 038, 042,
043, not all 44): nearly all are Prismal by this rule, since they are number
tools, graphs and learner-driven experiments (fraction walls, plant mass over
years with water, light or air turned off, seed spreading and crowding, heat
through materials). Only two have a clear SVF part:

- **RQ-PRISMAL-030**: the structure of the cut flower is SVF. The pollen and
  seed process is Prismal.
- **RQ-PRISMAL-016**: an everyday object zoomed into its particle view could be
  SVF if it is one fixed account. If the particles respond to temperature
  (RQ-015), that part is Prismal.

So SVF items will mostly come from areas **not yet written** (body systems,
cells, transport and breathing, bonding and structure), not from the current
requirements. The user already expects those areas to change as the tools prove out.

## The gap this reveals in SVF

The "predictable process" cases (water up a stem, atoms combining, growth
stages) all want the same thing: **a fixed sequence of changes over a
structure, written in a few steps, with the system doing the rest.** SVF has
named views but no time, sequence or per-part change of state. This is not one
lesson's need anymore.

**Decision (Claude, for the user to confirm):** do not design a sequence
capability yet. Build the flower with stage views, then one or two more items
that want a process, and record what authors had to spell out by hand. That
evidence decides what a "few steps" authoring surface must contain. The
question is open: *what is the smallest description of a fixed process over a
structure that a non-engineer author can write?*

## Overlap to watch

Prismal's stated scope is universal and educational (its D-017), and it already
has 3D views, timeline animation and a chemistry example. The boundary
therefore has to be kept from this side by the rule above. What SVF has that
Prismal does not: closed anatomical or molecular structures with named parts,
sectioning and peeling, adapted real source data with provenance and a stated
error.

## Not decided here

How the tools pass data, how Mava embeds either, and shared authoring words.
Deferred on purpose.
