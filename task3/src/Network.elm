module Network exposing
  ( Sim
  , addNode, removeNode, updateEdge, markRoute
  , distanceVector
  , animate, view
  )


import Html exposing (Html)
import Svg exposing (Svg)
import Svg.Attributes as SvgAttrib
import Point exposing (Point)
import Graph exposing (Graph)
import Visualised exposing (Positioned)
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
    shouldFilter edge =
      (edge.first.id == id1 && edge.second.id == id2) ||
        (edge.first.id == id2 && edge.second.id == id1)

    withoutEdge =
      Graph.filterFullEdges (not << shouldFilter) sim.network
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


animate : Float -> Point -> Sim -> Sim
animate timestep center sim =
  let
    updateGraph = Visualised.simulate timestep center
  in
    case sim of
      DistanceVector sim ->
        DistanceVector { sim | network = updateGraph sim.network }


simulationGraph : Sim -> Network ()
simulationGraph sim =
  let
    nodeData node =
      { pos = node.pos
      , v = node.v
      , a = node.a
      , id = node.id
      , data = ()
      }
  in
    case sim of
      DistanceVector sim ->
        Graph.mapNodes nodeData sim.network


view : Point -> Sim -> Html a
view size sim =
  let
    graph = simulationGraph sim

    points = viewPoints (Graph.nodes graph)

    edges = List.map viewEdge (Graph.edges graph)

    items = edges ++ points

    width = toString size.x

    height = toString size.y
  in
    Svg.svg
      [ SvgAttrib.viewBox <| "0 0 " ++ width ++ " " ++ height
      ]
      items



viewPoints : List (Positioned (Node ())) -> List (Svg a)
viewPoints =
  List.sortBy .id >> List.map viewPoint


viewPoint : Positioned (Node ()) -> Svg a
viewPoint point =
  Svg.g []
    [ Svg.circle
      [ SvgAttrib.cx <| toString point.pos.x
      , SvgAttrib.cy <| toString point.pos.y
      , SvgAttrib.r "15"
      , SvgAttrib.fill "white"
      , SvgAttrib.stroke "black"
      , SvgAttrib.strokeWidth "2"
      ]
      []
    , Svg.text_
      [ SvgAttrib.x <| toString point.pos.x
      , SvgAttrib.y <| toString point.pos.y
      , SvgAttrib.textAnchor "middle"
      , SvgAttrib.alignmentBaseline "middle"
      ]
      [ Svg.text point.id ]
    ]


viewEdge : { first : Positioned (Node ()), second : Positioned (Node ()), data : EdgeData } -> Svg a
viewEdge edge =
  Svg.line
    [ SvgAttrib.x1 <| toString edge.first.pos.x
    , SvgAttrib.y1 <| toString edge.first.pos.y
    , SvgAttrib.x2 <| toString edge.second.pos.x
    , SvgAttrib.y2 <| toString edge.second.pos.y
    , SvgAttrib.stroke "black"
    ]
    []
