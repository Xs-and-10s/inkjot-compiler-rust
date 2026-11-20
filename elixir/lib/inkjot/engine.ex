defmodule Inkjot.Engine do
  @moduledoc """
  The Inkjot engine: plays an Inklink's IR. See `docs/engine.md`.

      {:ok, inklink} = Inkjot.Engine.load(File.read!("ex3.json"))
      {:ok, state} = Inkjot.Engine.start(inklink, 42)
      %{links: [%{id: id} | _]} = Inkjot.Engine.view(state)
      {:ok, next} = Inkjot.Engine.take_link(state, id)
      {:ok, again} = Inkjot.Engine.replay(inklink, 42, %{}, Inkjot.Engine.actions(next))
      Inkjot.Engine.view(again) == Inkjot.Engine.view(next)

  - **States are values.** `take_link/2` returns a new state; the old one
    still works, so keep any state you want to branch from.
  - **A reading is its log.** The seed, the start and `actions/1` rebuild
    any state with `replay/5`.
  - **The same log reads the same way** under the same `engine_version/0`.
  - **Errors are data**: `{:error, {code, message, at}}`. A refused call
    changes nothing. A term of the wrong shape raises `ArgumentError`.

  The calls that play the story or read a whole file (`build/1`,
  `load/1`, `start/3`, `take_link/2`, `act/2`, `replay/5`, `replay_all/4`)
  run on dirty CPU schedulers. The calls that read a state (`view/1`,
  `events/1`, `meta/1`, `actions/1`) take microseconds and run on normal
  ones.
  """

  alias Inkjot.Native

  @typedoc "A loaded IR. A reference: cheap to pass, freed by the GC."
  @type inklink :: reference()
  @typedoc "A reading at one moment. A reference, never changed."
  @type state :: reference()

  @type start :: %{
          optional(:nib) => String.t(),
          optional(:pov) => String.t(),
          optional(:hints) => boolean()
        }
  @type action ::
          {:take_link, String.t()}
          | {:answer, [String.t()]}
          | {:vote, who :: String.t(), how :: String.t(), proof :: [String.t()]}

  @type code ::
          :bad_ir
          | :not_offered
          | :not_asking
          | :bad_answer
          | :step_limit
          | :unsupported
          | :story_error
          | :panic
  @type refused :: {:error, {code(), String.t(), non_neg_integer() | nil}}

  @type link :: %{id: String.t(), label: String.t()}
  @type ask :: %{
          kind: String.t(),
          prompt: String.t(),
          options: [String.t()],
          methods: [String.t()],
          length: non_neg_integer() | nil,
          voted: boolean()
        }
  @type view :: %{
          nib: String.t(),
          tags: [String.t()],
          pov: String.t(),
          now: non_neg_integer(),
          text: [String.t()],
          links: [link()],
          asks: [ask()],
          over: boolean()
        }
  @typedoc "An engine event, as the platform's log stores it: string keys."
  @type event :: %{String.t() => term()}

  @doc "The engine's version. Record it with every reading."
  @spec engine_version() :: pos_integer()
  defdelegate engine_version, to: Native

  @doc "Compile Inkjot source to IR JSON, or the errors `inkjot check` reports."
  @spec build(String.t()) :: {:ok, String.t()} | {:error, [String.t()]}
  defdelegate build(source), to: Native

  @doc "Read an IR from its JSON text."
  @spec load(binary()) :: {:ok, inklink()} | refused()
  defdelegate load(ir), to: Native

  @doc "Title, kind, mode, start, povs, playable characters, tags, versions."
  @spec meta(inklink()) :: map()
  defdelegate meta(inklink), to: Native

  @doc "Begin a reading. `start` may name a `:nib`, a `:pov`, and `hints: false`."
  @spec start(inklink(), non_neg_integer(), start()) :: {:ok, state()} | refused()
  def start(inklink, seed, start \\ %{}), do: Native.start(inklink, seed, start)

  @doc "What the Linker sees. Worked out once, when the state was made."
  @spec view(state()) :: view()
  defdelegate view(state), to: Native

  @doc "The engine's events from the action that made this state, in order."
  @spec events(state()) :: [event()]
  defdelegate events(state), to: Native

  @doc "The log: every action since the start."
  @spec actions(state()) :: [action()]
  defdelegate actions(state), to: Native

  @doc "The seed the reading began with."
  @spec seed(state()) :: non_neg_integer()
  defdelegate seed(state), to: Native

  @doc "The last finale this reading reached, or `nil`."
  @spec ending(state()) :: String.t() | nil
  defdelegate ending(state), to: Native

  @doc "Take an offered Link."
  @spec take_link(state(), String.t()) :: {:ok, state()} | refused()
  defdelegate take_link(state, id), to: Native

  @doc "Do one thing: take a Link, answer an input, or vote."
  @spec act(state(), action()) :: {:ok, state()} | refused()
  defdelegate act(state, action), to: Native

  @doc "Rebuild a reading from its log: the state after `upto` actions, or all."
  @spec replay(inklink(), non_neg_integer(), start(), [action()], non_neg_integer() | nil) ::
          {:ok, state()} | refused()
  def replay(inklink, seed, start, actions, upto \\ nil),
    do: Native.replay(inklink, seed, start, actions, upto)

  @doc "Every state of a reading: the start, then one per action."
  @spec replay_all(inklink(), non_neg_integer(), start(), [action()]) ::
          {:ok, [state()]} | refused()
  defdelegate replay_all(inklink, seed, start, actions), to: Native
end
