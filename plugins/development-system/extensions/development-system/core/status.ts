import { parseProjectPolicy, type ProjectPolicy } from "./configuration.ts";

export type HarnessMode = "tui" | "rpc" | "json" | "print";
export type StatusError = Readonly<{
  code: string;
  message: string;
  nextAction: string;
}>;
export type RepositoryIdentity = Readonly<{
  current: string;
  primary: string;
  kind: "primary" | "linked";
}>;
export type ComponentStatus = Readonly<{
  available: boolean;
  entrypoint: string;
  target: string | null;
  error?: string;
}>;
export type DevelopmentSystemStatus = Readonly<{
  configured: boolean;
  deliveryMode: ProjectPolicy["delivery"]["mode"] | null;
  enabledFeatures: readonly string[];
  checkout: RepositoryIdentity;
  components: Readonly<
    Record<"beads" | "development-discipline", ComponentStatus>
  >;
  enforcement: Readonly<{
    mode: HarnessMode;
    trustedApproval: boolean;
    limitations: readonly string[];
  }>;
  errors: readonly StatusError[];
  diagnostics: readonly string[];
}>;

export type StatusEffect =
  | Readonly<{ type: "resolve-repository" }>
  | Readonly<{ type: "read-policy"; primary: string }>
  | Readonly<{ type: "resolve-components" }>
  | Readonly<{ type: "run-doctor"; primary: string }>;

type State =
  | Readonly<{ tag: "waiting-repository" }>
  | Readonly<{
      tag: "waiting-policy";
      repository: RepositoryIdentity;
    }>
  | Readonly<{
      tag: "waiting-components";
      policy: ProjectPolicy | null;
      errors: readonly StatusError[];
      repository: RepositoryIdentity;
    }>
  | Readonly<{
      tag: "waiting-doctor";
      policy: ProjectPolicy | null;
      errors: readonly StatusError[];
      repository: RepositoryIdentity;
      components: DevelopmentSystemStatus["components"];
    }>
  | Readonly<{ tag: "done"; status: DevelopmentSystemStatus }>;

export type StatusStep =
  | Readonly<{ done: false; effect: StatusEffect }>
  | Readonly<{ done: true; value: DevelopmentSystemStatus }>;

/** Pure Step/Trampoline workflow. The interpreter owns all filesystem and process effects. */
export class StatusFlow {
  #state: State = { tag: "waiting-repository" };
  readonly #mode: HarnessMode;

  constructor(mode: HarnessMode) {
    this.#mode = mode;
  }

  step(): StatusStep {
    switch (this.#state.tag) {
      case "waiting-repository":
        return { done: false, effect: { type: "resolve-repository" } };
      case "waiting-policy":
        return {
          done: false,
          effect: {
            type: "read-policy",
            primary: this.#state.repository.primary,
          },
        };
      case "waiting-components":
        return { done: false, effect: { type: "resolve-components" } };
      case "waiting-doctor":
        return {
          done: false,
          effect: {
            type: "run-doctor",
            primary: this.#state.repository.primary,
          },
        };
      case "done":
        return { done: true, value: this.#state.status };
    }
  }

  resume(result: unknown): void {
    switch (this.#state.tag) {
      case "waiting-repository":
        this.#state = {
          tag: "waiting-policy",
          repository: result as RepositoryIdentity,
        };
        return;
      case "waiting-policy": {
        const policyResult = result as {
          source: string | null;
          error?: StatusError;
        };
        let policy: ProjectPolicy | null = null;
        const errors: StatusError[] = [];
        if (policyResult.source === null) {
          errors.push(
            policyResult.error ?? {
              code: "development_system.configuration_missing",
              message:
                ".development-system.toml is not present; delivery and mutation authority are unknown.",
              nextAction: "Run the setup workflow from the primary checkout.",
            },
          );
        } else {
          policy = parseProjectPolicy(policyResult.source);
        }
        this.#state = {
          tag: "waiting-components",
          policy,
          errors,
          repository: this.#state.repository,
        };
        return;
      }
      case "waiting-components":
        this.#state = {
          ...this.#state,
          tag: "waiting-doctor",
          components: result as DevelopmentSystemStatus["components"],
        };
        return;
      case "waiting-doctor": {
        const enabledFeatures = this.#state.policy
          ? [
              this.#state.policy.features.agenticSystems && "agentic-systems",
              this.#state.policy.features.evalCaseReporting &&
                "eval-case-reporting",
              this.#state.policy.features.beads && "beads",
              this.#state.policy.features.worktrees && "worktrees",
            ]
              .filter((value): value is string => Boolean(value))
              .sort()
          : [];
        const limitations =
          this.#mode === "rpc"
            ? [
                "RPC extension UI is not trusted approval provenance.",
                "Direct RPC bash is unsupported for guarded execution.",
              ]
            : this.#mode === "tui"
              ? []
              : ["This mode cannot grant trusted mutation approval."];
        this.#state = {
          tag: "done",
          status: Object.freeze({
            configured: this.#state.policy !== null,
            deliveryMode: this.#state.policy?.delivery.mode ?? null,
            enabledFeatures,
            checkout: this.#state.repository,
            components: this.#state.components,
            enforcement: Object.freeze({
              mode: this.#mode,
              trustedApproval: this.#mode === "tui",
              limitations,
            }),
            errors: this.#state.errors,
            diagnostics: result as readonly string[],
          }),
        };
        return;
      }
      case "done":
        throw new Error("development_system.status_already_complete");
    }
  }
}
