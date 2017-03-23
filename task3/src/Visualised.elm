module Visualised exposing (Visualised, Node, simulate)

import Graph exposing (Graph)
import Point exposing (Point, (.+), (.-), (.*), (./))


type alias Node n = { n | pos : Point, v : Point, a : Point }


type alias Visualised n e = Graph (Node n) e


repulsion : Float
repulsion = 100


damping : Float
damping = 0.7


scale : Float
scale = 100


simulate : Float -> Point -> Visualised n e -> Visualised n e
simulate timestep center graph =
  graph
  |> clearAcceleration
  |> applyCoulombsLaw
  |> applyHookesLaw
  |> attractToCenter center
  |> updateVelocity timestep
  |> updatePosition timestep


clearAcceleration : Visualised n e -> Visualised n e
clearAcceleration =
  Graph.mapNodes (\node -> { node | a = Point.zero })


applyCoulombsLaw : Visualised n e -> Visualised n e
applyCoulombsLaw graph =
  let
    distanceToForce pt =
      let
        len = max (scale / 10) (Point.length pt)
      in
        pt .* (repulsion / (len * len * len) * (scale * scale))

    force node =
      graph
      |> Graph.nodes
      |> List.map (\n -> distanceToForce (node.pos .- n.pos))
      |> List.foldl (.+) Point.zero

    updateNode node =
      { node | a = node.a .+ force node }
  in
    Graph.mapNodes updateNode graph


applyHookesLaw : Visualised n e -> Visualised n e
applyHookesLaw graph =
  let
    distanceToForce pt =
      let
        len = max 1 (Point.length pt)
        displacement = len - scale
      in
        (pt ./ len) .* (-displacement * 5)

    force node =
      graph
      |> Graph.neighbours (\n -> n == node)
      |> Maybe.withDefault []
      |> List.map (\(n, _) -> distanceToForce (node.pos .- n.pos))
      |> List.foldl (.+) Point.zero

    updateNode node =
      { node | a = node.a .+ force node }
  in
    Graph.mapNodes updateNode graph


attractToCenter : Point -> Visualised n e -> Visualised n e
attractToCenter center graph =
  let
    attraction pt = (pt .- center) .* (repulsion / -50)

    force node =
      attraction node.pos
    
    updateNode node =
      { node | a = node.a .+ force node }
  in
    Graph.mapNodes updateNode graph


updateVelocity : Float -> Visualised n e -> Visualised n e
updateVelocity timestep graph =
  let
    newVelocity v a = (v .+ (a .* timestep)) .* damping

    updateNode node =
      { node | v = newVelocity node.v node.a }
  in
    Graph.mapNodes updateNode graph


updatePosition : Float -> Visualised n e -> Visualised n e
updatePosition timestep graph =
  let
    newPosition pos v = pos .+ (v .* timestep)

    updateNode node =
      { node | pos = newPosition node.pos node.v }
  in
    Graph.mapNodes updateNode graph
