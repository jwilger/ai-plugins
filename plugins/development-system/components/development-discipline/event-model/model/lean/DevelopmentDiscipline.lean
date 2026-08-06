namespace DevelopmentDiscipline

-- EMC generated Lean4 model root.

def modelName := "development-discipline"

def modelVersion := "0.1.0"

def modelDigest := "ec9f3ed08c3d35df33dc3396a8433a5511f9100b149e681b9693dbc7e0816bed"

structure ModelWorkflow where
  workflow : String

def modelWorkflows : List ModelWorkflow := [{ workflow := "conduct-final-review" }]

structure ModelSlice where
  workflow : String
  slice : String

structure ModelSliceModule where
  workflow : String
  slice : String
  formalModule : String

structure ModelScenario where
  workflow : String
  slice : String
  scenarioKind : String
  scenario : String

structure ModelScenarioDefinition where
  workflow : String
  slice : String
  scenarioKind : String
  scenario : String
  given : String
  when : String
  thenStep : String
  readStreams : List String
  writtenStreams : List String
  contractKind : String
  coveredDefinition : String
  errorReferences : List String

inductive ModelDataFlowSourceKind where
  | original
  | modeledTarget
deriving BEq, DecidableEq, Repr

structure ModelDataFlow where
  workflow : String
  slice : String
  datum : String
  sourceKind : ModelDataFlowSourceKind
  source : String
  transformation : String
  target : String
  bitEncoding : String

structure ModelOutcome where
  workflow : String
  slice : String
  outcome : String
  events : List String
  externallyRelevant : Bool

structure ModelCommandError where
  workflow : String
  slice : String
  command : String
  error : String
  scenario : String
  recovery : String

structure ModelCommand where
  workflow : String
  slice : String
  command : String

inductive ModelCommandInputSourceKind where
  | actor
  | session
  | generated
  | externalPayload
  | eventStreamState
  | invocationArgument
deriving BEq, DecidableEq, Repr

structure ModelCommandInput where
  workflow : String
  slice : String
  command : String
  input : String
  sourceKind : ModelCommandInputSourceKind
  sourceDescription : String
  provenanceChain : List String
  eventStreamSourceEvent : String
  eventStreamSourceAttribute : String
  externalPayloadSourceName : String
  externalPayloadSourceField : String
  generatedSourceName : String
  generatedSourceField : String
  sessionSourceName : String
  sessionSourceField : String
  invocationArgumentSourceName : String
  invocationArgumentSourceField : String

structure ModelReadModel where
  workflow : String
  slice : String
  readModel : String

structure ModelReadModelDefinition where
  workflow : String
  slice : String
  readModel : String
  transitive : Bool
  relationshipFields : List String
  transitiveRule : String
  exampleScenarioName : String

structure ModelReadModelField where
  workflow : String
  slice : String
  readModel : String
  field : String
  sourceKind : String
  sourceEvent : String
  sourceAttribute : String
  derivationRule : String
  derivationSourceFields : List String
  absenceEvent : String
  derivationScenarioName : String
  absenceScenarioName : String
  provenance : String

structure ModelView where
  workflow : String
  slice : String
  view : String

structure ModelViewDefinition where
  workflow : String
  slice : String
  view : String
  readModels : List String
  sketchTokens : List String
  localStates : List String
  filters : List String

structure ModelViewControl where
  workflow : String
  slice : String
  view : String
  control : String
  command : String
  input : String
  inputSourceKind : ModelCommandInputSourceKind
  inputSourceDescription : String
  inputSketchToken : String
  inputVisibleToActor : Bool
  inputDecisionField : Bool
  handledErrors : List String
  recoveryBehavior : String
  controlSketchToken : String
  navigationType : String
  navigationTarget : String
  externalWorkflow : String
  externalSystem : String
  handoffContract : String

structure ModelViewField where
  workflow : String
  slice : String
  view : String
  field : String
  sourceKind : String
  sourceReadModel : String
  sourceField : String
  provenance : String
  bitEncoding : String

structure ModelBoardElement where
  workflow : String
  slice : String
  element : String
  kind : String
  lane : String
  declaredName : String
  mainPath : Bool

structure ModelBoardConnection where
  workflow : String
  slice : String
  source : String
  sourceKind : String
  target : String
  targetKind : String

structure ModelAutomation where
  workflow : String
  slice : String
  automation : String

structure ModelAutomationDefinition where
  workflow : String
  slice : String
  automation : String
  trigger : String
  command : String
  handledErrors : List String
  reaction : String

structure ModelTranslation where
  workflow : String
  slice : String
  translation : String

structure ModelTranslationDefinition where
  workflow : String
  slice : String
  translation : String
  externalEvent : String
  payloadContract : String
  command : String

structure ModelExternalPayload where
  workflow : String
  slice : String
  externalPayload : String

structure ModelExternalPayloadField where
  workflow : String
  slice : String
  externalPayload : String
  field : String
  provenance : String
  bitEncoding : String

structure ModelStream where
  workflow : String
  slice : String
  stream : String

structure ModelEvent where
  workflow : String
  slice : String
  event : String
  stream : String

structure ModelEventAttribute where
  workflow : String
  slice : String
  event : String
  attributeName : String
  sourceKind : String
  sourceName : String
  sourceField : String
  generatedSourceKind : String
  provenance : String

def modelSlices : List ModelSlice := [{ workflow := "conduct-final-review", slice := "manage-final-review" },{ workflow := "conduct-final-review", slice := "view-final-review" }]

def modelSliceModules : List ModelSliceModule := [{ workflow := "conduct-final-review", slice := "manage-final-review", formalModule := "ManageFinalReview" },{ workflow := "conduct-final-review", slice := "view-final-review", formalModule := "ViewFinalReview" }]

def modelSliceBelongsToDeclaredWorkflow (slice : ModelSlice) : Bool := modelWorkflows.any (fun workflow => workflow.workflow == slice.workflow)

def modelSliceHasModule (slice : ModelSlice) : Bool := modelSliceModules.any (fun sliceModule => sliceModule.workflow == slice.workflow && sliceModule.slice == slice.slice && sliceModule.formalModule.isEmpty == false)

def modelSliceModuleBelongsToDeclaredSlice (sliceModule : ModelSliceModule) : Bool := sliceModule.formalModule.isEmpty == false && modelSlices.any (fun slice => slice.workflow == sliceModule.workflow && slice.slice == sliceModule.slice)

def modelWorkflowSlicesHaveModules (workflow : ModelWorkflow) : Bool := modelSlices.all (fun slice => slice.workflow != workflow.workflow || modelSliceHasModule slice)

def modelWorkflowHasCompositionStructure (workflow : ModelWorkflow) : Bool := modelWorkflowSlicesHaveModules workflow

def modelScenarios : List ModelScenario := [{ workflow := "conduct-final-review", slice := "manage-final-review", scenarioKind := "acceptance", scenario := "Advance preserves every final review hold" },{ workflow := "conduct-final-review", slice := "manage-final-review", scenarioKind := "acceptance", scenario := "Plan creates resumable review session" },{ workflow := "conduct-final-review", slice := "manage-final-review", scenarioKind := "acceptance", scenario := "Scope split confirmation resumes review" },{ workflow := "conduct-final-review", slice := "manage-final-review", scenarioKind := "contract", scenario := "Addressed legacy session imports once" },{ workflow := "conduct-final-review", slice := "manage-final-review", scenarioKind := "contract", scenario := "Cooperative processes serialize execution" },{ workflow := "conduct-final-review", slice := "manage-final-review", scenarioKind := "contract", scenario := "Stale caller revision fails closed" },{ workflow := "conduct-final-review", slice := "view-final-review", scenarioKind := "acceptance", scenario := "Reports reconstruct from immutable events" },{ workflow := "conduct-final-review", slice := "view-final-review", scenarioKind := "contract", scenario := "Final review projector reconstructs exactly once" }]

def modelScenarioDefinitions : List ModelScenarioDefinition := [{ workflow := "conduct-final-review", slice := "manage-final-review", scenarioKind := "acceptance", scenario := "Advance preserves every final review hold", given := "an active review in planning iteration scope split delta risk verifier or budget decision state", when := "final_review.advance submits the existing caller state and revision", thenStep := "semantic events preserve every existing hold budget decision outcome and completion transition", readStreams := ["final-review:catalog","final-review:<session-id>"], writtenStreams := ["final-review:catalog","final-review:<session-id>"], contractKind := "", coveredDefinition := "", errorReferences := [] },{ workflow := "conduct-final-review", slice := "manage-final-review", scenarioKind := "acceptance", scenario := "Plan creates resumable review session", given := "a valid final review plan and no authoritative session", when := "final_review.plan executes", thenStep := "a review planned event creates catalog and session state while preserving response JSON and revision behavior", readStreams := ["final-review:catalog","final-review:<session-id>"], writtenStreams := ["final-review:catalog","final-review:<session-id>"], contractKind := "", coveredDefinition := "", errorReferences := [] },{ workflow := "conduct-final-review", slice := "manage-final-review", scenarioKind := "acceptance", scenario := "Scope split confirmation resumes review", given := "a review held for explicit scope split confirmation", when := "final_review.confirm_split supplies the current revision", thenStep := "a split confirmed event advances the existing FSM without changing public tool behavior", readStreams := ["final-review:catalog","final-review:<session-id>"], writtenStreams := ["final-review:catalog","final-review:<session-id>"], contractKind := "", coveredDefinition := "", errorReferences := [] },{ workflow := "conduct-final-review", slice := "manage-final-review", scenarioKind := "contract", scenario := "Addressed legacy session imports once", given := "an addressed session exists only in the legacy SQLite table", when := "that session is opened", thenStep := "one legacy review imported event is emitted idempotently without scanning other rows", readStreams := ["final-review:catalog","final-review:<session-id>"], writtenStreams := ["final-review:catalog","final-review:<session-id>"], contractKind := "command", coveredDefinition := "submit-review-command", errorReferences := [] },{ workflow := "conduct-final-review", slice := "manage-final-review", scenarioKind := "contract", scenario := "Cooperative processes serialize execution", given := "two local processes target the same SQLite artifact", when := "they execute final review commands concurrently", thenStep := "a short advisory lock serializes EventCore execution and expected versions prevent lost updates", readStreams := ["final-review:catalog","final-review:<session-id>"], writtenStreams := ["final-review:catalog","final-review:<session-id>"], contractKind := "command", coveredDefinition := "submit-review-command", errorReferences := [] },{ workflow := "conduct-final-review", slice := "manage-final-review", scenarioKind := "contract", scenario := "Stale caller revision fails closed", given := "the authoritative projected revision differs from caller carried state", when := "a review command executes", thenStep := "the existing stale revision error is returned and no event is appended", readStreams := ["final-review:catalog","final-review:<session-id>"], writtenStreams := ["final-review:catalog","final-review:<session-id>"], contractKind := "command", coveredDefinition := "submit-review-command", errorReferences := ["stale-review-revision"] },{ workflow := "conduct-final-review", slice := "view-final-review", scenarioKind := "acceptance", scenario := "Reports reconstruct from immutable events", given := "a retained or completed final review event stream", when := "status clean or out-of-scope reports are requested after restart", thenStep := "the existing report format state JSON revision and retention visibility are reconstructed from projections", readStreams := [], writtenStreams := [], contractKind := "", coveredDefinition := "", errorReferences := [] },{ workflow := "conduct-final-review", slice := "view-final-review", scenarioKind := "contract", scenario := "Final review projector reconstructs exactly once", given := "immutable semantic session events and an optional saved checkpoint", when := "the final review projector starts or resumes", thenStep := "state reports revisions and catalog visibility equal a single ordered fold after the checkpoint", readStreams := [], writtenStreams := [], contractKind := "projector", coveredDefinition := "final-review-state", errorReferences := [] }]

def modelDataFlows : List ModelDataFlow := [{ workflow := "conduct-final-review", slice := "manage-final-review", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed review-command payload for budget-decision-requested", transformation := "projection", target := "budget-decision-requested", bitEncoding := "UTF-8 JSON semantic review event payload" },{ workflow := "conduct-final-review", slice := "manage-final-review", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed review-command payload for budget-decision-resolved", transformation := "projection", target := "budget-decision-resolved", bitEncoding := "UTF-8 JSON semantic review event payload" },{ workflow := "conduct-final-review", slice := "manage-final-review", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed review-command payload for delta-risk-requested", transformation := "projection", target := "delta-risk-requested", bitEncoding := "UTF-8 JSON semantic review event payload" },{ workflow := "conduct-final-review", slice := "manage-final-review", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed review-command payload for iteration-accepted", transformation := "projection", target := "iteration-accepted", bitEncoding := "UTF-8 JSON semantic review event payload" },{ workflow := "conduct-final-review", slice := "manage-final-review", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed review-command payload for legacy-review-imported", transformation := "projection", target := "legacy-review-imported", bitEncoding := "UTF-8 JSON semantic review event payload" },{ workflow := "conduct-final-review", slice := "manage-final-review", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed review-command payload for review-catalog-updated", transformation := "projection", target := "review-catalog-updated", bitEncoding := "UTF-8 JSON semantic review event payload" },{ workflow := "conduct-final-review", slice := "manage-final-review", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed review-command payload for review-completed", transformation := "projection", target := "review-completed", bitEncoding := "UTF-8 JSON semantic review event payload" },{ workflow := "conduct-final-review", slice := "manage-final-review", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed review-command payload for review-planned", transformation := "projection", target := "review-planned", bitEncoding := "UTF-8 JSON semantic review event payload" },{ workflow := "conduct-final-review", slice := "manage-final-review", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed review-command payload for review-state-persisted", transformation := "projection", target := "review-state-persisted", bitEncoding := "UTF-8 JSON semantic review event payload" },{ workflow := "conduct-final-review", slice := "manage-final-review", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed review-command payload for scope-split-confirmed", transformation := "projection", target := "scope-split-confirmed", bitEncoding := "UTF-8 JSON semantic review event payload" },{ workflow := "conduct-final-review", slice := "manage-final-review", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed review-command payload for scope-split-held", transformation := "projection", target := "scope-split-held", bitEncoding := "UTF-8 JSON semantic review event payload" },{ workflow := "conduct-final-review", slice := "manage-final-review", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed review-command payload for verifier-requested", transformation := "projection", target := "verifier-requested", bitEncoding := "UTF-8 JSON semantic review event payload" },{ workflow := "conduct-final-review", slice := "manage-final-review", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "parsed review-command payload for verifier-resolved", transformation := "projection", target := "verifier-resolved", bitEncoding := "UTF-8 JSON semantic review event payload" },{ workflow := "conduct-final-review", slice := "manage-final-review", datum := "review-command", sourceKind := ModelDataFlowSourceKind.original, source := "MCP plan advance or confirm split request", transformation := "transformation", target := "submit-review-command", bitEncoding := "UTF-8 JSON preserving existing request and state schemas" },{ workflow := "conduct-final-review", slice := "view-final-review", datum := "payload", sourceKind := ModelDataFlowSourceKind.original, source := "persisted semantic review event bytes", transformation := "identity", target := "review-state-persisted", bitEncoding := "UTF-8 JSON semantic review event payload" },{ workflow := "conduct-final-review", slice := "view-final-review", datum := "review-command", sourceKind := ModelDataFlowSourceKind.original, source := "visible MCP review control input", transformation := "identity", target := "final-review-view", bitEncoding := "UTF-8 JSON review command" },{ workflow := "conduct-final-review", slice := "view-final-review", datum := "state", sourceKind := ModelDataFlowSourceKind.original, source := "ordered review event fold", transformation := "projection", target := "final-review-state", bitEncoding := "UTF-8 JSON preserving existing response schema" },{ workflow := "conduct-final-review", slice := "view-final-review", datum := "state", sourceKind := ModelDataFlowSourceKind.original, source := "ordered review event fold", transformation := "projection", target := "final-review-view", bitEncoding := "UTF-8 JSON preserving existing response schema" }]

def modelOutcomes : List ModelOutcome := [{ workflow := "conduct-final-review", slice := "manage-final-review", outcome := "review-state-confirmed", events := ["review-state-persisted"], externallyRelevant := false }]

def modelCommandErrors : List ModelCommandError := [{ workflow := "conduct-final-review", slice := "manage-final-review", command := "submit-review-command", error := "stale-review-revision", scenario := "Stale caller revision fails closed", recovery := "stay_on_screen" }]

def modelCommands : List ModelCommand := [{ workflow := "conduct-final-review", slice := "manage-final-review", command := "submit-review-command" }]

def modelCommandInputs : List ModelCommandInput := [{ workflow := "conduct-final-review", slice := "manage-final-review", command := "submit-review-command", input := "review-command", sourceKind := ModelCommandInputSourceKind.actor, sourceDescription := "parsed plan advance or confirm split request with caller carried state and revision", provenanceChain := ["MCP request","parse public schema","compare authoritative projection","pure EventCore command"], eventStreamSourceEvent := "", eventStreamSourceAttribute := "", externalPayloadSourceName := "", externalPayloadSourceField := "", generatedSourceName := "", generatedSourceField := "", sessionSourceName := "", sessionSourceField := "", invocationArgumentSourceName := "", invocationArgumentSourceField := "" }]

def modelReadModels : List ModelReadModel := [{ workflow := "conduct-final-review", slice := "view-final-review", readModel := "final-review-state" }]

def modelReadModelDefinitions : List ModelReadModelDefinition := [{ workflow := "conduct-final-review", slice := "view-final-review", readModel := "final-review-state", transitive := false, relationshipFields := [], transitiveRule := "", exampleScenarioName := "" }]

def modelReadModelFields : List ModelReadModelField := [{ workflow := "conduct-final-review", slice := "view-final-review", readModel := "final-review-state", field := "state", sourceKind := "event_attribute", sourceEvent := "review-state-persisted", sourceAttribute := "payload", derivationRule := "", derivationSourceFields := [], absenceEvent := "", derivationScenarioName := "", absenceScenarioName := "", provenance := "fold semantic session events in global position order" }]

def modelViews : List ModelView := [{ workflow := "conduct-final-review", slice := "view-final-review", view := "final-review-view" }]

def modelViewDefinitions : List ModelViewDefinition := [{ workflow := "conduct-final-review", slice := "view-final-review", view := "final-review-view", readModels := ["final-review-state"], sketchTokens := ["review-state-json"], localStates := [], filters := [] }]

def modelViewControls : List ModelViewControl := [{ workflow := "conduct-final-review", slice := "view-final-review", view := "final-review-view", control := "submit-review-command", command := "submit-review-command", input := "review-command", inputSourceKind := ModelCommandInputSourceKind.actor, inputSourceDescription := "plan advance or confirm split request", inputSketchToken := "review-command-json", inputVisibleToActor := true, inputDecisionField := true, handledErrors := ["stale-review-revision"], recoveryBehavior := "stay_on_screen", controlSketchToken := "submit-review-command", navigationType := "modeled_view", navigationTarget := "final-review-view", externalWorkflow := "", externalSystem := "", handoffContract := "" }]

def modelBoardElements : List ModelBoardElement := []

def modelBoardConnections : List ModelBoardConnection := []

def modelViewFields : List ModelViewField := [{ workflow := "conduct-final-review", slice := "view-final-review", view := "final-review-view", field := "state", sourceKind := "read_model", sourceReadModel := "final-review-state", sourceField := "state", provenance := "existing MCP state JSON clean report and out-of-scope report", bitEncoding := "UTF-8 JSON preserving existing response schema" }]

def modelAutomations : List ModelAutomation := []

def modelAutomationDefinitions : List ModelAutomationDefinition := []

def modelTranslations : List ModelTranslation := []

def modelTranslationDefinitions : List ModelTranslationDefinition := []

def modelExternalPayloads : List ModelExternalPayload := []

def modelExternalPayloadFields : List ModelExternalPayloadField := []

def modelStreams : List ModelStream := [{ workflow := "conduct-final-review", slice := "manage-final-review", stream := "final-review:<session-id>" },{ workflow := "conduct-final-review", slice := "manage-final-review", stream := "final-review:catalog" },{ workflow := "conduct-final-review", slice := "view-final-review", stream := "final-review:<session-id>" }]

def modelEvents : List ModelEvent := [{ workflow := "conduct-final-review", slice := "manage-final-review", event := "budget-decision-requested", stream := "final-review:<session-id>" },{ workflow := "conduct-final-review", slice := "manage-final-review", event := "budget-decision-resolved", stream := "final-review:<session-id>" },{ workflow := "conduct-final-review", slice := "manage-final-review", event := "delta-risk-requested", stream := "final-review:<session-id>" },{ workflow := "conduct-final-review", slice := "manage-final-review", event := "iteration-accepted", stream := "final-review:<session-id>" },{ workflow := "conduct-final-review", slice := "manage-final-review", event := "legacy-review-imported", stream := "final-review:<session-id>" },{ workflow := "conduct-final-review", slice := "manage-final-review", event := "review-catalog-updated", stream := "final-review:catalog" },{ workflow := "conduct-final-review", slice := "manage-final-review", event := "review-completed", stream := "final-review:<session-id>" },{ workflow := "conduct-final-review", slice := "manage-final-review", event := "review-planned", stream := "final-review:<session-id>" },{ workflow := "conduct-final-review", slice := "manage-final-review", event := "review-state-persisted", stream := "final-review:<session-id>" },{ workflow := "conduct-final-review", slice := "manage-final-review", event := "scope-split-confirmed", stream := "final-review:<session-id>" },{ workflow := "conduct-final-review", slice := "manage-final-review", event := "scope-split-held", stream := "final-review:<session-id>" },{ workflow := "conduct-final-review", slice := "manage-final-review", event := "verifier-requested", stream := "final-review:<session-id>" },{ workflow := "conduct-final-review", slice := "manage-final-review", event := "verifier-resolved", stream := "final-review:<session-id>" },{ workflow := "conduct-final-review", slice := "view-final-review", event := "review-state-persisted", stream := "final-review:<session-id>" }]

def modelEventAttributes : List ModelEventAttribute := [{ workflow := "conduct-final-review", slice := "manage-final-review", event := "budget-decision-requested", attributeName := "payload", sourceKind := "command_input", sourceName := "review-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed review command fact copied without reinterpretation" },{ workflow := "conduct-final-review", slice := "manage-final-review", event := "budget-decision-resolved", attributeName := "payload", sourceKind := "command_input", sourceName := "review-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed review command fact copied without reinterpretation" },{ workflow := "conduct-final-review", slice := "manage-final-review", event := "delta-risk-requested", attributeName := "payload", sourceKind := "command_input", sourceName := "review-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed review command fact copied without reinterpretation" },{ workflow := "conduct-final-review", slice := "manage-final-review", event := "iteration-accepted", attributeName := "payload", sourceKind := "command_input", sourceName := "review-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed review command fact copied without reinterpretation" },{ workflow := "conduct-final-review", slice := "manage-final-review", event := "legacy-review-imported", attributeName := "payload", sourceKind := "command_input", sourceName := "review-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed review command fact copied without reinterpretation" },{ workflow := "conduct-final-review", slice := "manage-final-review", event := "review-catalog-updated", attributeName := "payload", sourceKind := "command_input", sourceName := "review-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed review command fact copied without reinterpretation" },{ workflow := "conduct-final-review", slice := "manage-final-review", event := "review-completed", attributeName := "payload", sourceKind := "command_input", sourceName := "review-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed review command fact copied without reinterpretation" },{ workflow := "conduct-final-review", slice := "manage-final-review", event := "review-planned", attributeName := "payload", sourceKind := "command_input", sourceName := "review-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed review command fact copied without reinterpretation" },{ workflow := "conduct-final-review", slice := "manage-final-review", event := "review-state-persisted", attributeName := "payload", sourceKind := "command_input", sourceName := "review-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed review command fact copied without reinterpretation" },{ workflow := "conduct-final-review", slice := "manage-final-review", event := "scope-split-confirmed", attributeName := "payload", sourceKind := "command_input", sourceName := "review-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed review command fact copied without reinterpretation" },{ workflow := "conduct-final-review", slice := "manage-final-review", event := "scope-split-held", attributeName := "payload", sourceKind := "command_input", sourceName := "review-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed review command fact copied without reinterpretation" },{ workflow := "conduct-final-review", slice := "manage-final-review", event := "verifier-requested", attributeName := "payload", sourceKind := "command_input", sourceName := "review-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed review command fact copied without reinterpretation" },{ workflow := "conduct-final-review", slice := "manage-final-review", event := "verifier-resolved", attributeName := "payload", sourceKind := "command_input", sourceName := "review-command", sourceField := "payload", generatedSourceKind := "", provenance := "parsed review command fact copied without reinterpretation" },{ workflow := "conduct-final-review", slice := "view-final-review", event := "review-state-persisted", attributeName := "payload", sourceKind := "generated", sourceName := "event-store", sourceField := "payload", generatedSourceKind := "event_replay", provenance := "immutable semantic review event replayed by projector" }]

def modelScenarioDefinitionHasGwt (scenario : ModelScenarioDefinition) : Bool := scenario.given.isEmpty == false && scenario.when.isEmpty == false && scenario.thenStep.isEmpty == false

def modelScenarioKindIsFirstClass (scenario : ModelScenarioDefinition) : Bool := scenario.scenarioKind == "acceptance" || scenario.scenarioKind == "contract"

def modelDataFlowIsBitComplete (dataFlow : ModelDataFlow) : Bool := dataFlow.datum.isEmpty == false && dataFlow.source.isEmpty == false && dataFlow.transformation.isEmpty == false && dataFlow.target.isEmpty == false && dataFlow.bitEncoding.isEmpty == false

def modelDataFlowCoversDatumTarget (workflow : String) (slice : String) (datum : String) (target : String) : Bool := modelDataFlows.any (fun dataFlow => dataFlow.workflow == workflow && dataFlow.slice == slice && dataFlow.datum == datum && dataFlow.target == target && modelDataFlowIsBitComplete dataFlow)

def modelDataFlowBitEncodingMatchesDatumTarget (workflow : String) (slice : String) (datum : String) (target : String) (bitEncoding : String) : Bool := modelDataFlows.any (fun dataFlow => dataFlow.workflow == workflow && dataFlow.slice == slice && dataFlow.datum == datum && dataFlow.target == target && dataFlow.bitEncoding == bitEncoding && modelDataFlowIsBitComplete dataFlow)

def modelDataFlowSourceBitEncodingMatchesModeledSource (dataFlow : ModelDataFlow) : Bool := (modelDataFlows.any (fun sourceFlow => sourceFlow.workflow == dataFlow.workflow && sourceFlow.slice == dataFlow.slice && sourceFlow.datum == dataFlow.datum && sourceFlow.target == dataFlow.source) == false) || modelDataFlows.any (fun sourceFlow => sourceFlow.workflow == dataFlow.workflow && sourceFlow.slice == dataFlow.slice && sourceFlow.datum == dataFlow.datum && sourceFlow.target == dataFlow.source && sourceFlow.bitEncoding == dataFlow.bitEncoding && modelDataFlowIsBitComplete sourceFlow)

def modelDataFlowHasModeledTransformationSemantics (dataFlow : ModelDataFlow) : Bool := dataFlow.transformation == "identity" || dataFlow.transformation == "projection" || dataFlow.transformation == "derivation" || dataFlow.transformation == "default" || dataFlow.transformation == "absence" || dataFlow.transformation == "transformation"

def modelDataFlowHasModeledSourceKind (dataFlow : ModelDataFlow) : Bool := match dataFlow.sourceKind with
  | ModelDataFlowSourceKind.original => dataFlow.source.isEmpty == false
  | ModelDataFlowSourceKind.modeledTarget => dataFlow.source.isEmpty == false

def modelDataFlowModeledSourceResolves (dataFlow : ModelDataFlow) : Bool := dataFlow.sourceKind != ModelDataFlowSourceKind.modeledTarget || modelDataFlows.any (fun sourceFlow => sourceFlow.workflow == dataFlow.workflow && sourceFlow.slice == dataFlow.slice && sourceFlow.datum == dataFlow.datum && sourceFlow.target == dataFlow.source && modelDataFlowIsBitComplete sourceFlow)

def modelSameDataFlowTarget (left : ModelDataFlow) (right : ModelDataFlow) : Bool := left.workflow == right.workflow && left.slice == right.slice && left.datum == right.datum && left.target == right.target

def modelDataFlowTargetsFromReachable (reachable : List ModelDataFlow) : List ModelDataFlow := modelDataFlows.filter (fun dataFlow => dataFlow.sourceKind == ModelDataFlowSourceKind.modeledTarget && reachable.any (fun sourceFlow => sourceFlow.workflow == dataFlow.workflow && sourceFlow.slice == dataFlow.slice && sourceFlow.datum == dataFlow.datum && sourceFlow.target == dataFlow.source && modelDataFlowIsBitComplete sourceFlow))

def modelDataFlowsReachableFromOriginalsAfterFuel : Nat -> List ModelDataFlow -> List ModelDataFlow
  | Nat.zero, reachable => reachable
  | Nat.succ fuel, reachable => modelDataFlowsReachableFromOriginalsAfterFuel fuel (reachable ++ modelDataFlowTargetsFromReachable reachable)

def modelDataFlowsReachableFromOriginals : List ModelDataFlow := modelDataFlowsReachableFromOriginalsAfterFuel modelDataFlows.length (modelDataFlows.filter (fun dataFlow => dataFlow.sourceKind == ModelDataFlowSourceKind.original && modelDataFlowIsBitComplete dataFlow))

def modelDataFlowHasOriginalSourceChain (dataFlow : ModelDataFlow) : Bool := dataFlow.sourceKind == ModelDataFlowSourceKind.original || modelDataFlowsReachableFromOriginals.any (fun reachableFlow => modelSameDataFlowTarget reachableFlow dataFlow)

def modelDataFlowTargetsFromBitPreservingReachable (reachable : List ModelDataFlow) : List ModelDataFlow := modelDataFlows.filter (fun dataFlow => dataFlow.sourceKind == ModelDataFlowSourceKind.modeledTarget && reachable.any (fun sourceFlow => sourceFlow.workflow == dataFlow.workflow && sourceFlow.slice == dataFlow.slice && sourceFlow.datum == dataFlow.datum && sourceFlow.target == dataFlow.source && sourceFlow.bitEncoding == dataFlow.bitEncoding && modelDataFlowIsBitComplete sourceFlow))

def modelDataFlowsReachableFromOriginalsWithPreservedBitsAfterFuel : Nat -> List ModelDataFlow -> List ModelDataFlow
  | Nat.zero, reachable => reachable
  | Nat.succ fuel, reachable => modelDataFlowsReachableFromOriginalsWithPreservedBitsAfterFuel fuel (reachable ++ modelDataFlowTargetsFromBitPreservingReachable reachable)

def modelDataFlowsReachableFromOriginalsWithPreservedBits : List ModelDataFlow := modelDataFlowsReachableFromOriginalsWithPreservedBitsAfterFuel modelDataFlows.length (modelDataFlows.filter (fun dataFlow => dataFlow.sourceKind == ModelDataFlowSourceKind.original && modelDataFlowIsBitComplete dataFlow))

def modelDataFlowHasBitPreservingOriginalSourceChain (dataFlow : ModelDataFlow) : Bool := dataFlow.sourceKind == ModelDataFlowSourceKind.original || modelDataFlowsReachableFromOriginalsWithPreservedBits.any (fun reachableFlow => modelSameDataFlowTarget reachableFlow dataFlow)

def modelCommandInputHasModeledDataFlow (input : ModelCommandInput) : Bool := modelDataFlowCoversDatumTarget input.workflow input.slice input.input input.command

def modelEventAttributeHasModeledDataFlow (eventAttribute : ModelEventAttribute) : Bool := modelDataFlowCoversDatumTarget eventAttribute.workflow eventAttribute.slice eventAttribute.attributeName eventAttribute.event

def modelReadModelFieldHasModeledDataFlow (field : ModelReadModelField) : Bool := modelDataFlowCoversDatumTarget field.workflow field.slice field.field field.readModel

def modelViewFieldHasModeledDataFlow (field : ModelViewField) : Bool := modelDataFlowCoversDatumTarget field.workflow field.slice field.field field.view

def modelViewFieldBitEncodingMatchesDataFlow (field : ModelViewField) : Bool := modelDataFlowBitEncodingMatchesDatumTarget field.workflow field.slice field.field field.view field.bitEncoding

def modelExternalPayloadFieldHasModeledDataFlow (field : ModelExternalPayloadField) : Bool := modelDataFlowCoversDatumTarget field.workflow field.slice field.field field.externalPayload

def modelExternalPayloadFieldBitEncodingMatchesDataFlow (field : ModelExternalPayloadField) : Bool := modelDataFlowBitEncodingMatchesDatumTarget field.workflow field.slice field.field field.externalPayload field.bitEncoding

def modelMeaningfulDataHasModeledDataFlows : Bool := modelCommandInputs.all modelCommandInputHasModeledDataFlow && modelEventAttributes.all modelEventAttributeHasModeledDataFlow && modelReadModelFields.all modelReadModelFieldHasModeledDataFlow && modelViewFields.all modelViewFieldHasModeledDataFlow && modelExternalPayloadFields.all modelExternalPayloadFieldHasModeledDataFlow

def modelCommandInputHasProvenance (input : ModelCommandInput) : Bool := input.sourceDescription.isEmpty == false && input.provenanceChain.isEmpty == false

def modelCommandInputTracesToInvocationSource (input : ModelCommandInput) : Bool := input.sourceKind == ModelCommandInputSourceKind.actor || (input.sourceKind == ModelCommandInputSourceKind.eventStreamState && input.eventStreamSourceEvent.isEmpty == false && input.eventStreamSourceAttribute.isEmpty == false) || (input.sourceKind == ModelCommandInputSourceKind.externalPayload && input.externalPayloadSourceName.isEmpty == false && input.externalPayloadSourceField.isEmpty == false) || (input.sourceKind == ModelCommandInputSourceKind.generated && input.generatedSourceName.isEmpty == false && input.generatedSourceField.isEmpty == false) || (input.sourceKind == ModelCommandInputSourceKind.session && input.sessionSourceName.isEmpty == false && input.sessionSourceField.isEmpty == false) || (input.sourceKind == ModelCommandInputSourceKind.invocationArgument && input.invocationArgumentSourceName.isEmpty == false && input.invocationArgumentSourceField.isEmpty == false)

def modelEventAttributeSourceIsComplete (eventAttribute : ModelEventAttribute) : Bool := eventAttribute.provenance.isEmpty == false && ((eventAttribute.sourceKind == "command_input" && eventAttribute.sourceName.isEmpty == false && eventAttribute.sourceField.isEmpty == false) || (eventAttribute.sourceKind == "external_payload" && eventAttribute.sourceName.isEmpty == false && eventAttribute.sourceField.isEmpty == false) || (eventAttribute.sourceKind == "generated" && eventAttribute.sourceName.isEmpty == false && eventAttribute.generatedSourceKind.isEmpty == false) || (eventAttribute.sourceKind == "session" && eventAttribute.sourceName.isEmpty == false) || (eventAttribute.sourceKind == "derivation" && eventAttribute.sourceName.isEmpty == false && eventAttribute.sourceField.isEmpty == false))

def modelReadModelFieldSourceIsComplete (field : ModelReadModelField) : Bool := (field.sourceKind == "event_attribute" && field.sourceEvent.isEmpty == false && field.sourceAttribute.isEmpty == false) || (field.sourceKind == "derivation" && field.derivationRule.isEmpty == false && field.derivationSourceFields.isEmpty == false) || (field.sourceKind == "absence_default" && field.absenceEvent.isEmpty == false)

def modelReadModelFieldTracesToOriginalProvenance (field : ModelReadModelField) : Bool := field.provenance.isEmpty == false && ((field.sourceKind == "event_attribute" && modelEventAttributes.any (fun eventAttribute => eventAttribute.workflow == field.workflow && eventAttribute.slice == field.slice && eventAttribute.event == field.sourceEvent && eventAttribute.attributeName == field.sourceAttribute && modelEventAttributeSourceIsComplete eventAttribute)) || (field.sourceKind == "derivation" && field.derivationRule.isEmpty == false && field.derivationSourceFields.isEmpty == false) || (field.sourceKind == "absence_default" && field.absenceEvent.isEmpty == false))

def modelViewFieldSourceIsComplete (field : ModelViewField) : Bool := field.sourceKind == "read_model" && field.sourceReadModel.isEmpty == false && field.sourceField.isEmpty == false && field.provenance.isEmpty == false && field.bitEncoding.isEmpty == false

def modelViewFieldReadModelFieldSourceResolves (viewField : ModelViewField) : Bool := modelViewFieldSourceIsComplete viewField && modelReadModelFields.any (fun readModelField => readModelField.workflow == viewField.workflow && readModelField.slice == viewField.slice && readModelField.readModel == viewField.sourceReadModel && readModelField.field == viewField.sourceField && modelReadModelFieldSourceIsComplete readModelField)

def modelDisplayedDatumTracesToOriginalProvenance (viewField : ModelViewField) : Bool := modelViewFieldReadModelFieldSourceResolves viewField && modelReadModelFields.any (fun readModelField => readModelField.workflow == viewField.workflow && readModelField.slice == viewField.slice && readModelField.readModel == viewField.sourceReadModel && readModelField.field == viewField.sourceField && modelReadModelFieldTracesToOriginalProvenance readModelField)

def modelExternalPayloadFieldHasProvenance (field : ModelExternalPayloadField) : Bool := field.provenance.isEmpty == false && field.bitEncoding.isEmpty == false

def modelControlProvidesCommandInput (control : ModelViewControl) (input : ModelCommandInput) : Bool := control.workflow == input.workflow && control.command == input.command && control.input == input.input

def modelViewControlProvidesEveryCommandInput (control : ModelViewControl) : Bool := modelCommandInputs.all (fun input => input.workflow != control.workflow || input.command != control.command || modelViewControls.any (fun providedInput => providedInput.workflow == control.workflow && providedInput.slice == control.slice && providedInput.view == control.view && providedInput.control == control.control && providedInput.command == control.command && modelControlProvidesCommandInput providedInput input))

def modelOutcomeBranchIsModeled (outcome : ModelOutcome) : Bool := outcome.outcome.isEmpty == false && outcome.events.isEmpty == false

def modelCommandErrorRecoveryIsModeled (commandError : ModelCommandError) : Bool := commandError.command.isEmpty == false && commandError.error.isEmpty == false && commandError.scenario.isEmpty == false && commandError.recovery.isEmpty == false

def modelViewControlNavigationTargetIsModeled (control : ModelViewControl) : Bool := control.navigationType.isEmpty || ((control.navigationType == "modeled_view" || control.navigationType == "local_view_state") && control.navigationTarget.isEmpty == false) || (control.navigationType == "external_workflow" && control.externalWorkflow.isEmpty == false) || (control.navigationType == "external_system" && control.externalSystem.isEmpty == false && control.handoffContract.isEmpty == false)

def modelExternalBoundaryContractIsModeled (translation : ModelTranslationDefinition) : Bool := translation.translation.isEmpty == false && translation.externalEvent.isEmpty == false && translation.payloadContract.isEmpty == false && translation.command.isEmpty == false

def modelWorkflowBehaviorSurfaceIsComplete : Bool := modelOutcomes.all modelOutcomeBranchIsModeled && modelCommandErrors.all modelCommandErrorRecoveryIsModeled && modelViewControls.all modelViewControlNavigationTargetIsModeled && modelTranslationDefinitions.all modelExternalBoundaryContractIsModeled

theorem modelIdentityIsStable : modelName = "development-discipline" := rfl

theorem modelVersionIsStable : modelVersion = "0.1.0" := rfl

theorem modelDigestIsStable : modelDigest = "ec9f3ed08c3d35df33dc3396a8433a5511f9100b149e681b9693dbc7e0816bed" := rfl

theorem modelWorkflowsAreDeclared : modelWorkflows.length = 1 := rfl

theorem modelSlicesAreDeclared : modelSlices.length = 2 := rfl

theorem modelSliceModulesAreDeclared : modelSliceModules.length = 2 := rfl

theorem modelWorkflowCompositionStructureComplete : (modelSlices.all modelSliceBelongsToDeclaredWorkflow && modelSlices.all modelSliceHasModule && modelSliceModules.all modelSliceModuleBelongsToDeclaredSlice && modelWorkflows.all modelWorkflowHasCompositionStructure) = true := by native_decide

theorem modelWorkflowBehaviorSurfaceIsCompleteIsStable : modelWorkflowBehaviorSurfaceIsComplete = true := by native_decide

theorem modelScenariosAreDeclared : modelScenarios.length = 8 := rfl

theorem modelScenarioDefinitionsAreDeclared : modelScenarioDefinitions.length = 8 := rfl

theorem modelScenarioDefinitionsHaveGwt : modelScenarioDefinitions.all modelScenarioDefinitionHasGwt = true := by native_decide

theorem modelScenarioKindsAreFirstClass : modelScenarioDefinitions.all modelScenarioKindIsFirstClass = true := by native_decide

theorem modelDataFlowsAreDeclared : modelDataFlows.length = 18 := rfl

theorem modelDataFlowsAreBitComplete : modelDataFlows.all modelDataFlowIsBitComplete = true := by native_decide

theorem modelDataFlowSourceKindsAreModeled : modelDataFlows.all modelDataFlowHasModeledSourceKind = true := by native_decide

theorem modelDataFlowModeledSourcesResolve : modelDataFlows.all modelDataFlowModeledSourceResolves = true := by native_decide

theorem modelDataFlowSourceChainsReachOriginals : modelDataFlows.all modelDataFlowHasOriginalSourceChain = true := by native_decide

theorem modelDataFlowSourceChainsPreserveBitEncodingSemantics : modelDataFlows.all modelDataFlowHasBitPreservingOriginalSourceChain = true := by native_decide

theorem modelDataFlowTransformationsAreModeled : modelDataFlows.all modelDataFlowHasModeledTransformationSemantics = true := by native_decide

theorem modelMeaningfulDataFlowsAreCovered : modelMeaningfulDataHasModeledDataFlows = true := by native_decide

theorem modelDataFlowSourceBitEncodingsMatchModeledSources : modelDataFlows.all modelDataFlowSourceBitEncodingMatchesModeledSource = true := by native_decide

theorem modelViewFieldBitEncodingsMatchDataFlows : modelViewFields.all modelViewFieldBitEncodingMatchesDataFlow = true := by native_decide

theorem modelExternalPayloadFieldBitEncodingsMatchDataFlows : modelExternalPayloadFields.all modelExternalPayloadFieldBitEncodingMatchesDataFlow = true := by native_decide

theorem modelOutcomesAreDeclared : modelOutcomes.length = 1 := rfl

theorem modelCommandErrorsAreDeclared : modelCommandErrors.length = 1 := rfl

theorem modelCommandsAreDeclared : modelCommands.length = 1 := rfl

theorem modelCommandInputsAreDeclared : modelCommandInputs.length = 1 := rfl

theorem modelCommandInputsHaveProvenance : modelCommandInputs.all modelCommandInputHasProvenance = true := by native_decide

theorem modelCommandInputsTraceToInvocationSources : modelCommandInputs.all modelCommandInputTracesToInvocationSource = true := by native_decide

theorem modelReadModelsAreDeclared : modelReadModels.length = 1 := rfl

theorem modelReadModelDefinitionsAreDeclared : modelReadModelDefinitions.length = 1 := rfl

theorem modelReadModelFieldsAreDeclared : modelReadModelFields.length = 1 := rfl

theorem modelEventAttributeSourcesAreComplete : modelEventAttributes.all modelEventAttributeSourceIsComplete = true := by native_decide

theorem modelReadModelFieldSourcesAreComplete : modelReadModelFields.all modelReadModelFieldSourceIsComplete = true := by native_decide

theorem modelViewFieldSourcesAreComplete : modelViewFields.all modelViewFieldSourceIsComplete = true := by native_decide

theorem modelViewFieldReadModelFieldSourcesResolve : modelViewFields.all modelViewFieldReadModelFieldSourceResolves = true := by native_decide

theorem modelDisplayedDataTraceToOriginalProvenance : modelViewFields.all modelDisplayedDatumTracesToOriginalProvenance = true := by native_decide

theorem modelExternalPayloadFieldsHaveProvenance : modelExternalPayloadFields.all modelExternalPayloadFieldHasProvenance = true := by native_decide

theorem modelViewsAreDeclared : modelViews.length = 1 := rfl

theorem modelViewDefinitionsAreDeclared : modelViewDefinitions.length = 1 := rfl

theorem modelViewControlsAreDeclared : modelViewControls.length = 1 := rfl

theorem modelViewControlsProvideCommandInputs : modelViewControls.all modelViewControlProvidesEveryCommandInput = true := by native_decide

theorem modelBoardElementsAreDeclared : modelBoardElements.length = 0 := rfl

theorem modelBoardConnectionsAreDeclared : modelBoardConnections.length = 0 := rfl

theorem modelViewFieldsAreDeclared : modelViewFields.length = 1 := rfl

theorem modelAutomationsAreDeclared : modelAutomations.length = 0 := rfl

theorem modelAutomationDefinitionsAreDeclared : modelAutomationDefinitions.length = 0 := rfl

theorem modelTranslationsAreDeclared : modelTranslations.length = 0 := rfl

theorem modelTranslationDefinitionsAreDeclared : modelTranslationDefinitions.length = 0 := rfl

theorem modelExternalPayloadsAreDeclared : modelExternalPayloads.length = 0 := rfl

theorem modelExternalPayloadFieldsAreDeclared : modelExternalPayloadFields.length = 0 := rfl

theorem modelStreamsAreDeclared : modelStreams.length = 3 := rfl

theorem modelEventsAreDeclared : modelEvents.length = 14 := rfl

theorem modelEventAttributesAreDeclared : modelEventAttributes.length = 14 := rfl

end DevelopmentDiscipline
