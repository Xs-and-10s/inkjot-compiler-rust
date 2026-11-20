defmodule Inkjot.Native do
  @moduledoc false
  # The NIF itself (`native/inkjot_nif`). Call `Inkjot.Engine` instead.
  #
  # Always built in release mode: the engine's step limit keeps its stack
  # small in release, and a debug build needs several times more, which a
  # scheduler thread may not have.

  use Rustler,
    otp_app: :inkjot,
    crate: "inkjot_nif",
    path: "native/inkjot_nif",
    mode: :release

  def engine_version, do: err()
  def build(_source), do: err()
  def load(_ir), do: err()
  def meta(_inklink), do: err()
  def start(_inklink, _seed, _start), do: err()
  def view(_state), do: err()
  def events(_state), do: err()
  def actions(_state), do: err()
  def seed(_state), do: err()
  def ending(_state), do: err()
  def act(_state, _action), do: err()
  def take_link(_state, _id), do: err()
  def replay(_inklink, _seed, _start, _actions, _upto), do: err()
  def replay_all(_inklink, _seed, _start, _actions), do: err()

  defp err, do: :erlang.nif_error(:nif_not_loaded)
end
