import { createMemo, createSignal, Index, Show } from "solid-js";
import { compile_regex, type Nfa } from "automaton_trial";

export default function App() {
  const [regex, setRegex] = createSignal("");

  const compiled = createMemo(() => {
    try {
      return compile_regex(regex());
    } catch (e) {
      return `${e}`;
    }
  });

  const stateMachines = () => {
    const m = compiled();
    if (typeof m === "string") {
      return undefined;
    } else {
      return m;
    }
  };

  return (
    <div class="size-full overflow-hidden flex flex-col items-center">
      <div class="p-5 w-full md:w-[48rem] flex flex-col gap-2 items-center overflow-y-auto overflow-x-hidden">
        <RegexInputArea value={regex()} onInput={setRegex} />
        <NfaDisplayArea label="Compiled NFA" nfa={stateMachines()?.nfa} />
        <NfaDisplayArea label="Converted DFA" nfa={stateMachines()?.dfa} />
      </div>
    </div>
  );
}

function RegexInputArea(
  props: { value: string; onInput: (regex: string) => void },
) {
  return (
    <div class="w-full flex flex-col main-area">
      <label for="regex-input">
        Input regex:
      </label>
      <input
        id="regex-input"
        type="text"
        class="my-1"
        value={props.value}
        oninput={(e) => props.onInput(e.currentTarget.value)}
      />
    </div>
  );
}

function NfaDisplayArea(props: { label: string; nfa: Nfa | undefined }) {
  return (
    <div class="flex flex-col w-full main-area items-stretch">
      <h2>{props.label}</h2>
      <Show when={props.nfa !== undefined}>
        <div class="flex flex-col items-center">
          <div class="max-w-full max-h-full overflow-y-hidden overflow-x-auto">
            <NfaGraph nfa={props.nfa as Nfa} />
          </div>
        </div>
      </Show>
    </div>
  );
}

type Vec2 = { readonly x: number; readonly y: number };
function NfaGraph(props: { nfa: Nfa }) {
  const pixelRate = 3;
  const r = 10;
  const gap = 10;
  const arrowSize = 4;
  const margin = 8;

  const posMap = createMemo(() => {
    return calcPos(props.nfa);
  });

  const layout = createMemo(() => {
    let maxX = 0;
    let maxY = 0;
    for (const { x, y } of posMap().values()) {
      maxX = Math.max(maxX, x);
      maxY = Math.max(maxY, y);
    }
    const x = mapPos(maxX) + r + gap + 2 * margin;
    const y = mapPos(maxY) + r + gap + 2 * margin;

    return {
      width: (x + margin) * pixelRate,
      height: (y + margin) * pixelRate,
      viewBox: `${-margin} ${-margin} ${x} ${y}`,
    };
  });

  const edges = () => {
    return props.nfa.states.flatMap((s, from) => {
      const eps = s.epsilon_transitions.map((to) => ({
        from,
        to,
        c: "",
      }));

      const e = Object.entries(s.branches).flatMap(([c, s]) => {
        return s.map((to) => ({ from, to, c }));
      });

      return eps.concat(e);
    });
  };

  function mapPos(pos: number): number {
    return (r + gap) * (2 * pos + 1);
  }

  return (
    <svg
      width={layout().width}
      height={layout().height}
      viewBox={layout().viewBox}
      preserveAspectRatio="xMidYMid meet"
    >
      <defs>
        <circle
          id="initial_state"
          fill="blue"
          stroke="black"
          r={r}
        >
        </circle>
        <circle id="normal_state" fill="white" stroke="black" r={r}></circle>
        <circle
          id="accept_mark"
          fill="transparent"
          stroke="black"
          r={r * 0.8}
        >
        </circle>
      </defs>

      <Index each={props.nfa.states}>
        {(state, index) => {
          const cx = () => {
            const x = posMap().get(index)!.x;
            return mapPos(x);
          };
          const cy = () => {
            const y = posMap().get(index)!.y;
            return mapPos(y);
          };

          const href = () => {
            return index === 0 ? "#initial_state" : "#normal_state";
          };

          return (
            <>
              <use href={href()} x={cx()} y={cy()} />
              <Show when={state().accepts}>
                <use href="#accept_mark" x={cx()} y={cy()} />
              </Show>
            </>
          );
        }}
      </Index>

      <Index each={edges()}>
        {(edge, _) => {
          const points = createMemo(() => {
            return calcPoints(edge().from, edge().to);
          });

          const commands = () => {
            const { sx, sy, cx1, cy1, cx2, cy2, ex, ey } = points();
            return `M ${sx} ${sy} C ${cx1} ${cy1} ${cx2} ${cy2} ${ex} ${ey}`;
          };

          const arrow = () => {
            const { cx2, cy2, ex, ey } = points();

            const p1 = dir(ex, ey, cx2, cy2);
            const nx = -p1.y * arrowSize / 2;
            const ny = p1.x * arrowSize / 2;

            const mx = p1.x * arrowSize * Math.SQRT1_2;
            const my = p1.y * arrowSize * Math.SQRT1_2;

            return `M ${ex} ${ey} l ${mx + nx} ${my + ny} l ${-nx * 2} ${
              -ny * 2
            } z`;
          };

          const labelPos = () => {
            const { cx1, cy1, cx2, cy2 } = points();
            return { x: (cx1 + cx2) / 2, y: (cy1 + cy2) / 2 };
          };

          return (
            <>
              <path
                d={commands()}
                stroke="black"
                fill="transparent"
              />
              <path
                d={arrow()}
                fill="black"
              />
              <text
                x={labelPos().x}
                y={labelPos().y}
                stroke="black"
                text-anchor="middle"
                dominant-baseline="middle"
              >
                {edge().c}
              </text>
            </>
          );
        }}
      </Index>
    </svg>
  );

  function calcPoints(from_id: number, to_id: number) {
    const from = posMap().get(from_id)!;
    const to = posMap().get(to_id)!;

    const fx = mapPos(from.x);
    const fy = mapPos(from.y);
    const tx = mapPos(to.x);
    const ty = mapPos(to.y);

    const { cx1, cy1, cy2, cx2 } = (() => {
      if (from_id === to_id) {
        const cx1 = fx + Math.SQRT1_2 * r * 3;
        const cy1 = fy - Math.SQRT1_2 * r * 3;
        const cx2 = fx + Math.SQRT1_2 * r * 3;
        const cy2 = fy + Math.SQRT1_2 * r * 3;
        return { cx1, cy1, cy2, cx2 };
      } else {
        const dx = tx - fx;
        const dy = ty - fy;
        const c = 1.5 * r / Math.hypot(dx, dy);
        const nx = dy * c;
        const ny = -dx * c;
        const cx1 = fx * 0.75 + tx * 0.25 + nx;
        const cy1 = fy * 0.75 + ty * 0.25 + ny;
        const cx2 = fx * 0.25 + tx * 0.75 + nx;
        const cy2 = fy * 0.25 + ty * 0.75 + ny;
        return { cx1, cy1, cy2, cx2 };
      }
    })();

    const { sx, sy } = (() => {
      const { x, y } = dir(fx, fy, cx1, cy1);
      const sx = fx + x * r;
      const sy = fy + y * r;
      return { sx, sy };
    })();

    const { ex, ey } = (() => {
      const { x, y } = dir(tx, ty, cx2, cy2);
      const ex = tx + x * r;
      const ey = ty + y * r;
      return { ex, ey };
    })();

    return { sx, sy, cx1, cy1, cx2, cy2, ex, ey };
  }

  function dir(sx: number, sy: number, ex: number, ey: number): Vec2 {
    const dx = ex - sx;
    const dy = ey - sy;
    const len = Math.hypot(dx, dy);
    const x = dx / len;
    const y = dy / len;
    return { x, y };
  }

  function calcPos(nfa: Nfa): Map<number, Vec2> {
    const positionMap = new Map<number, Vec2>();
    const stack = [0];
    positionMap.set(0, { x: 0, y: 0 });

    while (true) {
      const id = stack.pop();
      if (id === undefined) break;
      const { x: sx, y: sy } = positionMap.get(id)!;
      const state = nfa.states[id];
      const chars = Object.keys(state.branches).sort();

      const x = sx + 1;
      let y = sy;
      for (const c of chars) {
        for (const id of state.branches[c]) {
          if (positionMap.has(id)) {
            continue;
          }
          positionMap.set(id, { x, y });
          y += 1;
          stack.push(id);
        }
      }

      for (const id of state.epsilon_transitions) {
        if (positionMap.has(id)) {
          continue;
        }
        positionMap.set(id, { x, y });
        y += 1;
        stack.push(id);
      }
    }

    return positionMap;
  }
}
