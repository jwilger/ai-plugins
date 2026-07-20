# Delivery workflow benchmark workspace

This inert workspace is used to evaluate delivery-policy decisions. Scenarios
write their structured decision to `delivery-plan.json`; they must not perform
remote actions. `verify-delivery-plan.mjs` rejects plans that violate the mode,
CI, or failed-run-hold contract.
