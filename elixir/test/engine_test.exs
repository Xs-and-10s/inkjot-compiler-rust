defmodule Inkjot.EngineTest do
  @moduledoc """
  The engine through the NIF, as the platform will call it: states are
  values, a reading is its log, the same log reads the same way, events
  replay exactly, errors are data, and nothing a story does can take down
  the VM.
  """
  use ExUnit.Case, async: true
  use ExUnitProperties

  alias Inkjot.Engine

  @examples ~w(ex1-the-night-train ex2-the-ninefold-cell ex3-the-fall-of-the-old-regime)

  setup_all do
    inklinks =
      for name <- @examples, into: %{} do
        {:ok, inklink} = Engine.load(File.read!("examples/ir/#{name}.json"))
        {name, inklink}
      end

    %{inklinks: inklinks}
  end

  # Where a random reading may begin, as in the Rust tests.
  @starts [
    {"ex1-the-night-train", %{nib: "cast"}},
    {"ex1-the-night-train", %{nib: "dining@1", pov: "wren"}},
    {"ex1-the-night-train", %{nib: "sleeper@3", pov: "rook"}},
    {"ex2-the-ninefold-cell", %{}},
    {"ex2-the-ninefold-cell", %{nib: "orrery", pov: "eldric"}},
    {"ex2-the-ninefold-cell", %{nib: "ossuary", pov: "roland"}},
    {"ex3-the-fall-of-the-old-regime", %{}}
  ]

  # Something a Linker could do, chosen by `pick`: answer what is asked
  # (always when nothing else is offered), or take a Link.
  defp something(state, pick) do
    view = Engine.view(state)

    case {view.asks, view.links} do
      {[ask | _], links} when links == [] or rem(pick, 3) == 0 ->
        answer(ask, pick) || link(links, pick)

      {_, links} ->
        link(links, pick)
    end
  end

  defp answer(%{kind: "name input"}, _pick), do: {:answer, ["Ash"]}

  defp answer(%{kind: "choose input", options: [_ | _] = options, length: n}, pick) do
    words = for i <- 0..((n || 1) - 1), do: Enum.at(options, rem(pick + i * 7, length(options)))
    {:answer, words}
  end

  defp answer(%{kind: "accuse", voted: false, options: [_ | _] = who, methods: how}, pick) do
    how = if how == [], do: "", else: Enum.at(how, rem(pick, length(how)))
    {:vote, Enum.at(who, rem(pick, length(who))), how, []}
  end

  defp answer(_ask, _pick), do: nil

  defp link([], _pick), do: nil
  defp link(links, pick), do: {:take_link, Enum.at(links, rem(pick, length(links))).id}

  # Play by `picks`, keeping every state. Stops at the end, or at a feature
  # the interpreter does not run yet.
  defp play(inklink, seed, start, picks) do
    {:ok, first} = Engine.start(inklink, seed, start)

    Enum.reduce_while(picks, [first], fn pick, [now | _] = states ->
      case something(now, pick) do
        nil ->
          {:halt, states}

        action ->
          case Engine.act(now, action) do
            {:ok, next} -> {:cont, [next | states]}
            {:error, {:unsupported, _, _}} -> {:halt, states}
          end
      end
    end)
    |> Enum.reverse()
  end

  defp reading do
    gen all(
          start <- StreamData.member_of(@starts),
          seed <- StreamData.integer(1..1000),
          picks <- StreamData.list_of(StreamData.integer(0..19), min_length: 1, max_length: 30)
        ) do
      {start, seed, picks}
    end
  end

  describe "a reading" do
    property "replays to the same views and the same events", %{inklinks: inklinks} do
      check all({{name, start}, seed, picks} <- reading(), max_runs: 40) do
        inklink = inklinks[name]
        live = play(inklink, seed, start, picks)
        log = Engine.actions(List.last(live))
        {:ok, replayed} = Engine.replay_all(inklink, seed, start, log)

        assert length(replayed) == length(live)

        for {a, b} <- Enum.zip(live, replayed) do
          assert Engine.view(a) == Engine.view(b)
          assert Engine.events(a) == Engine.events(b)
        end

        k = rem(seed, length(live))
        {:ok, partway} = Engine.replay(inklink, seed, start, log, k)
        assert Engine.view(partway) == Engine.view(Enum.at(live, k))
      end
    end

    property "never changes a state it acted on", %{inklinks: inklinks} do
      check all({{name, start}, seed, picks} <- reading(), max_runs: 40) do
        states = play(inklinks[name], seed, start, picks)

        for [before, after_] <- Enum.chunk_every(states, 2, 1, :discard) do
          seen = Engine.view(before)
          {:ok, again} = Engine.act(before, List.last(Engine.actions(after_)))
          assert Engine.view(before) == seen
          assert Engine.view(again) == Engine.view(after_)
        end
      end
    end

    property "refuses a Link it does not offer, and changes nothing", %{inklinks: inklinks} do
      check all(
              {{name, start}, seed, picks} <- reading(),
              bogus <- StreamData.string(:alphanumeric, min_length: 1),
              max_runs: 40
            ) do
        state = List.last(play(inklinks[name], seed, start, picks))
        seen = Engine.view(state)

        unless Enum.any?(seen.links, &(&1.id == bogus)) do
          assert {:error, {:not_offered, _, nil}} = Engine.take_link(state, bogus)
        end

        assert Engine.view(state) == seen
      end
    end

    property "reports only events in the catalogue, as the log stores them", %{inklinks: inklinks} do
      check all({{name, start}, seed, picks} <- reading(), max_runs: 40) do
        for state <- play(inklinks[name], seed, start, picks), event <- Engine.events(state) do
          assert %{"type" => type, "type_version" => 1, "origin" => "engine", "data" => data} =
                   event

          assert is_map(data) and Enum.all?(Map.keys(data), &is_binary/1)

          assert type in ~w(nib_shown ending_reached reading_finished checkpoint_reached died
                            rewound echo_ended turn_ended)
        end
      end
    end
  end

  test "meta describes an Inklink before anyone reads it", %{inklinks: inklinks} do
    meta = Engine.meta(inklinks["ex3-the-fall-of-the-old-regime"])
    assert %{title: "The Fall of the Old Regime", kind: "story", mode: "solo", ir: "3"} = meta
    assert meta.povs == ["skittles"]
    assert meta.engine == Engine.engine_version()
  end

  test "a reading of Ex 3 ends at its finale", %{inklinks: inklinks} do
    {:ok, state} = Engine.start(inklinks["ex3-the-fall-of-the-old-regime"], 7)

    last =
      Stream.unfold(state, fn s ->
        case Engine.view(s).links do
          [] -> nil
          links -> {s, elem(Engine.take_link(s, List.last(links).id), 1)}
        end
      end)
      |> Enum.to_list()
      |> List.last()
      |> then(&elem(Engine.take_link(&1, List.last(Engine.view(&1).links).id), 1))

    assert Engine.view(last).over
    assert Engine.ending(last) == "the_list"

    assert Enum.map(Engine.events(last), & &1["type"]) ==
             ~w(nib_shown ending_reached reading_finished)
  end

  test "bad IR is refused, not raised" do
    assert {:error, {:bad_ir, _, nil}} = Engine.load("{")
    assert {:error, {:bad_ir, _, nil}} = Engine.load(<<0xFF, 0xFE>>)

    assert {:error, {:bad_ir, _, nil}} =
             Engine.load(~s({"ir": "2", "config": [], "passages": []}))
  end

  test "a term of the wrong shape raises", %{inklinks: inklinks} do
    {:ok, state} = Engine.start(inklinks["ex3-the-fall-of-the-old-regime"], 1)
    assert_raise ArgumentError, fn -> Engine.act(state, {:jump, "there"}) end

    assert_raise ArgumentError, fn ->
      Engine.start(inklinks["ex3-the-fall-of-the-old-regime"], -1)
    end
  end

  test "build compiles source, or says what is wrong" do
    assert {:ok, ir} = Engine.build(File.read!("examples/ex3-the-fall-of-the-old-regime.twee"))
    assert {:ok, _} = Engine.load(ir)

    assert {:error, [message | _]} =
             Engine.build(":: StoryTitle\nBroken\n\n:: start\n> [[Go->nowhere]]\n")

    assert message =~ "error"
  end

  @round_and_round """
  :: StoryTitle
  Round and Round

  :: StoryData
  {"ifid": "x", "format": "Inkjot", "format-version": "0.6.0", "start": "hall"}

  :: Inklink [config]
  kind: story
  mode: solo
  pov: ann
  character ann: "Ann" ~ playable

  :: hall
  The hall.
  > [[Go round->left]]

  :: left
  next: right
  --
  Left.

  :: right
  next: left
  --
  Right.
  """

  test "a next: loop is refused, and the VM lives" do
    {:ok, ir} = Engine.build(@round_and_round)
    {:ok, inklink} = Engine.load(ir)
    {:ok, hall} = Engine.start(inklink, 1)
    [%{id: id}] = Engine.view(hall).links

    assert {:error, {:step_limit, message, nil}} = Engine.take_link(hall, id)
    assert message =~ "step limit"
    assert Engine.view(hall).nib == "hall"
  end

  test "taking a Link stays under a millisecond, deep into the slowest example",
       %{inklinks: inklinks} do
    inklink = inklinks["ex1-the-night-train"]
    states = play(inklink, 3, %{nib: "dining@1", pov: "wren"}, Enum.to_list(1..40))
    state = List.last(Enum.reject(states, &(Engine.view(&1).links == [])))
    %{links: [%{id: id} | _]} = Engine.view(state)

    times =
      for _ <- 1..200 do
        {micros, {:ok, _}} = :timer.tc(fn -> Engine.take_link(state, id) end)
        micros
      end

    median = Enum.at(Enum.sort(times), 100)
    assert median < 1_000, "take_link took #{median} µs (median of 200)"
  end
end
