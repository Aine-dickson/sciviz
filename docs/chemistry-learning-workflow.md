# Chemistry learning workflow: molecular structure

Status: proposed educational workflow; design only. This document does not authorize renderer, Explorer, simulation, or lesson-authoring implementation.

Related work:
- [Experimental chemistry composition](experimental-chemistry-composition.md)
- [Chemistry composition API](chemistry-composition-api.md)
- [Chemistry molecular views](chemistry-molecular-views.md)

## 1. Purpose

Turn the existing chemistry composition model and ball-and-stick adapter into a useful educational experience. The target is not merely to display a molecule. Learners should build an accurate mental model of:

1. **Composition** — which atoms and elements are present.
2. **Connectivity** — which atoms are bonded to which other atoms, and the bond order represented by the model.
3. **Spatial arrangement** — how the connected atoms are arranged in three dimensions.
4. **Representation limits** — which aspects are scientific data and which are visual conventions chosen to make the structure understandable.

The experience should support the user's intended progression from teacher-led demonstration at lower levels, through guided exploration, to independent investigation at higher levels. The exact grade or syllabus placement remains to be selected against the user's curriculum.

## 2. First learning case: inspect and compare water and methane

Use water (H₂O) and methane (CH₄) as the first pair because they exercise different compositions and spatial arrangements while remaining small enough for a learner to inspect.

A learner should be able to answer, using the model rather than guesswork:

- Which elements make up this molecule?
- How many atom instances are present?
- Which atoms are connected?
- How many bonds are incident to a selected atom?
- What is the represented bond order?
- How does this molecule's arrangement differ from the other example?
- What does the model show, and what does it deliberately not show?

The examples are structural teaching models. They are not simulations of molecular motion, reactions, electron distributions, or chemical stability.

## 3. Learning progression

### A. Teacher-led demonstration

The teacher controls the pace and reveals one idea at a time:

1. Show the complete molecule with a clear formula and element labels.
2. Identify each atom instance and distinguish it from the reusable element definition.
3. Reveal bonds and trace connectivity.
4. Rotate the model to show that the arrangement is three-dimensional.
5. Compare water and methane, asking learners to count and describe before revealing the relevant information.

The learner's role is initially observation and verbal explanation. Interactions should help the teacher direct attention, not require every learner to operate the model independently.

### B. Guided exploration

The learner manipulates a prepared molecule while following a prompt:

1. Predict the number and types of atoms from the formula.
2. Select an atom and inspect its element identity.
3. Highlight its directly connected atoms and bonds.
4. Select a bond and inspect its endpoints and represented order.
5. Change the viewing angle and explain how the same structure looks from another direction.
6. Repeat for the second molecule and compare.

The model should distinguish direct neighbors from all other atoms. Selection and highlighting must be driven by chemical identity and connectivity, not by proximity in screen space.

### C. Independent investigation

At a more advanced level, learners use the model to support their own questions:

- Compare molecular composition and connectivity.
- Inspect the three-dimensional arrangement from multiple viewpoints.
- Explain why a flat drawing can omit spatial information.
- Identify the limits of the chosen representation and avoid treating display geometry as literal atomic boundaries.

Do not add bond-angle or bond-length measurement tasks until the example coordinates have an authoritative scientific basis and the displayed quantities have explicit measurement semantics.

## 4. Required interaction semantics

These are workflow requirements, not claims about current implementation.

| Interaction | Required scientific meaning |
|---|---|
| Select an atom | Resolve to its exact `AtomId` and element definition. |
| Highlight atom neighbors | Use molecule bond connectivity; do not infer neighbors from geometric distance. |
| Select a bond | Resolve to its exact `BondId`, endpoint `AtomId` values, and `BondOrder`. |
| Rotate / zoom | Change only the viewing transform; never mutate molecular coordinates. |
| Reset view | Restore a useful camera framing without changing the molecule. |
| Show / hide labels | Change presentation only; preserve all scientific entities and relationships. |
| Compare molecules | Keep each molecule's identity, composition, and connectivity distinct. |

A selection should be reversible and should not modify the source molecule. Visual emphasis must not obscure or replace the underlying atom/bond identity.

## 5. Scientific and visual integrity

The current ball-and-stick style uses illustrative display radii. Those radii are presentation parameters, not measured atomic radii or boundaries. Bond axes are visual encodings of semantic bonds, not literal electron density or complete descriptions of bonding.

Before using a molecular example to teach quantitative geometry:

- Record the source and status of its coordinates.
- Distinguish measured, computed, idealized, and illustrative coordinates.
- State units explicitly (the chemistry model uses ångströms).
- Only expose bond lengths or angles as scientific measurements when the source data and calculation semantics justify those claims.

Do not depict electrons as planets on fixed classical orbits. Do not infer valence, stability, polarity, or reaction behaviour from the current limited composition model.

## 6. Responsibility boundaries

- **SVF:** authoritative structural identity, atom/bond relationships, derived molecular representations, scientific selection and inspection, and view-independent interaction semantics.
- **Prismal:** dynamic scientific behaviour and computation when the learning objective actually requires simulation. Static molecular inspection does not need to be forced through Prismal.
- **Mava:** lesson sequencing, teacher/learner presentation, prompts, reveal steps, and assessment flow. It should consume scientific interaction capabilities rather than duplicate chemistry rules.

These are working boundaries to test, not a commitment to implement all three systems in the next increment.

## 7. Deliberate non-goals for this workflow

- Molecular dynamics, force fields, reaction pathways, or energy calculations.
- Electron orbitals or electron-density rendering.
- Arbitrary molecule construction or chemical authoring UI.
- General-purpose scene editing.
- A generic interaction scripting framework.
- A second molecular representation before the first learning task is validated.
- Automatic extension into biology or unrelated scientific domains.

## 8. Decision gate before implementation

Do not automatically translate every interaction above into an immediate feature list. First choose one concrete lesson/task from the user's curriculum and validate:

1. The learner level and prerequisite knowledge.
2. The learning objective and evidence that the learner has understood it.
3. Whether the task is teacher-led, guided, or independent.
4. Which of the interactions above are essential for that task.
5. Whether the molecule's coordinates and scientific claims are suitable for that level.
6. Which layer owns each required capability.

The first implementation should support one complete learning task end to end. Add only the capabilities that task requires. A renderer/Explorer integration is a likely delivery path for visual interaction, but it should be scoped from the selected task rather than assumed to be the next milestone.

## 9. Exit criteria for this design study

The workflow is ready to become an implementation contract when one lesson is selected, its learner outcomes and interaction sequence are explicit, the scientific status of its example data is documented, and every required capability has a clear owner among SVF, Prismal, and Mava.
