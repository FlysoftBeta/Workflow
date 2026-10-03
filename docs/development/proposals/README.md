# Proposing future work

A proposal is a reviewable description of a change that has not yet become the product or implementation contract. Use one when a feature changes user behavior, a module boundary, persistent state, the wire protocol, or the verification matrix. Small fixes that fit the current contract can stay in their task packet.

Start from the [proposal template](template.md). Explain the user problem before the mechanism, identify the behavior that would count as success, and make the important trade-offs visible. Name the code owners and contract changes so work can be divided into independent tasks. An implementation plan should describe dependency order and acceptance, not narrate every coding step.

A draft can be accepted, revised, superseded, or declined. Keep that state and the decision rationale in the proposal. Once implemented, update the product, UX, and implementation documents, link the evidence, and mark the proposal implemented. Preserve its original motivation; the maintained documents become the source of current behavior. Do not present an accepted proposal as tested functionality before the required checks run.

No remote transport implementation is approved by the 1.0.0 architecture. A future SSH feature would require a separate proposal and user authorization; the existing bootstrap interface alone does not authorize building the connection.
