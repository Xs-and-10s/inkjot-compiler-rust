defmodule Inkjot.MixProject do
  use Mix.Project

  # The Elixir side of the Inkjot engine: a NIF over `native/inkjot_nif`,
  # for the Inklinks platform. The Rust compiler and engine live at the
  # root of this repository; the Elixir code lives under `elixir/`.
  def project do
    [
      app: :inkjot,
      version: "0.5.0",
      elixir: "~> 1.15",
      elixirc_paths: ["elixir/lib"],
      test_paths: ["elixir/test"],
      deps: deps(),
      description: "The Inkjot engine, as a NIF, for the Inklinks platform",
      source_url: "https://github.com/Xs-and-10s/inkjot-compiler-rust"
    ]
  end

  def application do
    [extra_applications: []]
  end

  defp deps do
    [
      {:rustler, "~> 0.38.0", runtime: false},
      {:stream_data, "~> 1.1", only: :test}
    ]
  end
end
