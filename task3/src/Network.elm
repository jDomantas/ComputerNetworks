module Network exposing (Sim)


import Point
import Graph exposing (Graph)
import NetworkCommon exposing (..)
import DistanceVector


addNodeHelper : NodeId -> Simulation a b -> Simulation a b
addNodeHelper id sim =
  let
    newNode =
      { id = id
      , data = sim.init
      , pos = Point.zero
      , v = Point.zero
      , a = Point.zero
      }
  in
    if Graph.anyNode (\n -> n.id == id) sim.network then
      sim
    else
      { sim | network = Graph.addNode newNode sim.network }


removeNodeHelper : NodeId -> Simulation a b -> Simulation a b
removeNodeHelper id sim =
  { sim | network = Graph.filterNodes (\n -> n.id /= id) sim.network }


updateEdgeHelper : NodeId -> NodeId -> Maybe Int -> Simulation a b -> Simulation a b
updateEdgeHelper id1 id2 cost sim =
  let
    filter edge =
      (edge.first.id == id1 && edge.second.id == id2) ||
        (edge.first.id == id2 && edge.second.id == id1)

    withoutEdge =
      Graph.filterFullEdges filter sim.network
  in
    case cost of
      Just cost ->
        let
          withUpdatedEdge =
            Graph.connect
              (\n -> n.id == id1)
              (\n -> n.id == id2)
              { cost = cost, lastTravel = Nothing }
              withoutEdge
        in
          { sim | network = withUpdatedEdge }

      Nothing ->
        { sim | network = withoutEdge }


markRouteHelper : NodeId -> NodeId -> Int -> Simulation a b -> Simulation a b
markRouteHelper start end time sim = Debug.crash "not implemented"


type Sim
  = DistanceVector DistanceVector.Sim


addNode : NodeId -> Sim -> Sim
addNode id sim =
  case sim of
    DistanceVector sim ->
      DistanceVector (addNodeHelper id sim)


removeNode : NodeId -> Sim -> Sim
removeNode id sim =
  case sim of
    DistanceVector sim ->
      DistanceVector (removeNodeHelper id sim)


updateEdge : NodeId -> NodeId -> Maybe Int -> Sim -> Sim
updateEdge id1 id2 cost sim =
  case sim of
    DistanceVector sim ->
      DistanceVector (updateEdgeHelper id1 id2 cost sim)


markRoute : NodeId -> NodeId -> Int -> Sim -> Sim
markRoute start end time sim =
  case sim of
    DistanceVector sim ->
      DistanceVector (markRouteHelper start end time sim)


distanceVector : Sim
distanceVector =
  DistanceVector
    { network = Graph.empty
    , init = DistanceVector.init
    , update = DistanceVector.update
    , announce = DistanceVector.announce
    , route = DistanceVector.route
    , disconnect = DistanceVector.disconnect
    }
