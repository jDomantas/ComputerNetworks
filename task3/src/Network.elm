module Network exposing
  ( Sim
  , addNode, removeNode, updateEdge, markRoute, viewNode
  , distanceVector
  , update, animate, view
  )


import Html exposing (Html)
import Html.Attributes
import Svg exposing (Svg)
import Svg.Attributes as SvgAttrib
import List.Extra
import Point exposing (Point, (.+), (.-), (.*), (./))
import Graph exposing (Graph)
import Visualised exposing (Positioned)
import NetworkCommon exposing (..)
import DistanceVector


addNodeHelper : NodeId -> Simulation a b -> Simulation a b
addNodeHelper id sim =
  let
    newNode =
      { id = id
      , data = sim.init id
      -- a hack to shift nodes 'randomly', to prevent stacking
      , pos = Point 0 (10 * (toFloat <| List.length <| Graph.nodes sim.network))
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
  let
    neighbours =
      Graph.neighbours (\n -> n.id == id) sim.network
      |> Maybe.withDefault []
      |> List.map (\(node, _) -> node.id)
    
    -- remove neighbours before with edge removal function,
    -- to correctly announce disconnects
    withoutNeighbours =
      List.foldl (\n sim -> updateEdgeHelper id n Nothing sim) sim neighbours
  in
    { sim | network = Graph.filterNodes (\n -> n.id /= id) withoutNeighbours.network }


updateEdgeHelper : NodeId -> NodeId -> Maybe Int -> Simulation a b -> Simulation a b
updateEdgeHelper id1 id2 cost sim =
  let
    shouldFilter edge =
      (edge.first.id == id1 && edge.second.id == id2) ||
        (edge.first.id == id2 && edge.second.id == id1)

    withoutEdge =
      Graph.filterFullEdges (not << shouldFilter) sim.network

    hasRemoved =
      List.length (Graph.edges withoutEdge) < List.length (Graph.edges sim.network)
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
        let
          network =
            if hasRemoved then
              Graph.mapNodes (\node ->
                if node.id == id1 then
                  { node | data = sim.disconnect id2 node.data }
                else if node.id == id2 then
                  { node | data = sim.disconnect id1 node.data }
                else
                  node) withoutEdge
            else
              withoutEdge
        in
          { sim | network = network }


markRouteHelper : NodeId -> NodeId -> Int -> Simulation a b -> Simulation a b
markRouteHelper start end time sim =
  let
    markEdge : NodeId -> NodeId -> Simulation a b -> Simulation a b
    markEdge from to sim =
      let
        graph =
          Graph.mapFullEdges (\edge ->
            if (edge.first.id == from && edge.second.id == to) ||
              (edge.first.id == to && edge.second.id == from) then
              { cost = edge.data.cost, lastTravel = Just time }
            else
              edge.data) sim.network
      in
        { sim | network = graph }

    mark : NodeId -> NodeId -> Int -> Simulation a b -> Simulation a b
    mark start end steps sim =
      if steps == 0 || start == end then
        sim
      else
        let
          current =
            Graph.findNode (\n -> n.id == start) sim.network
          
          nextHop =
            current
            |> Maybe.map .data
            |> Maybe.andThen (flip sim.route end)
        in
          Maybe.map3 markEdge (Maybe.map .id current) nextHop (Just sim)
          |> Maybe.map4 mark nextHop (Just end) (Just (steps - 1))
          |> Maybe.withDefault sim
  in
    mark start end 10 sim


sendMessages : NodeId -> Simulation a b -> Simulation a b
sendMessages source sim =
  let
    msg =
      Graph.findNode (\n -> n.id == source) sim.network
      |> Maybe.map (\node -> sim.announce node.data)

    receivers =
      Graph.neighbours (\n -> n.id == source) sim.network
      |> Maybe.withDefault []
      |> List.filterMap (\(node, edge) ->
        msg |> Maybe.map (\payload ->
          let
            msg =
              { data = payload
              , sender = source
              , cost = edge.cost
              }
          in
            (msg, node.id)))

    updated =
      List.foldl (\(msg, recv) sim ->
        let
          updated =
            Graph.mapNodes (\node ->
              if node.id == recv then
                { node | data = sim.update msg node.data }
              else
                node) sim.network
        in
          { sim | network = updated }) sim receivers
  in
    updated


viewNodeHelper : NodeId -> Simulation a b -> Maybe (List String)
viewNodeHelper id sim =
  Graph.findNode (\n -> n.id == id) sim.network
  |> Maybe.map (\node -> sim.view node.data)


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


viewNode : NodeId -> Sim -> Maybe (List String)
viewNode node sim =
  case sim of
    DistanceVector sim ->
      viewNodeHelper node sim


distanceVector : Sim
distanceVector =
  DistanceVector
    { network = Graph.empty
    , init = DistanceVector.init
    , update = DistanceVector.update
    , announce = DistanceVector.announce
    , route = DistanceVector.route
    , disconnect = DistanceVector.disconnect
    , view = DistanceVector.view
    }


update : Int -> Sim -> Sim
update randomValue sim =
  let
    pickRandom list =
      let
        index = randomValue % (max 1 (List.length list))
      in
        List.Extra.getAt index list

    send sim =
      Graph.nodes sim.network
      |> pickRandom
      |> Maybe.map (\source -> sendMessages source.id sim)
      |> Maybe.withDefault sim
  in
    case sim of
      DistanceVector sim ->
        DistanceVector (send sim)


animate : Float -> Point -> Sim -> Sim
animate timestep center sim =
  let
    updateVisual sim =
      { sim | network = Visualised.simulate timestep center sim.network }
  in
    case sim of
      DistanceVector sim ->
        DistanceVector (updateVisual sim)


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


view : Point -> Int -> Maybe NodeId -> Maybe NodeId -> Sim -> Html a
view size tick start end sim =
  let
    graph = simulationGraph sim

    pickType id =
      if Just id == start then
        Start
      else if Just id == end then
        End
      else
        Normal

    points = viewPoints pickType (Graph.nodes graph)

    edges = List.map (viewEdge tick) (Graph.edges graph)

    items = edges ++ points

    width = toString size.x

    height = toString size.y
  in
    Html.div
      [ Html.Attributes.class "graph " ]
      [ Svg.svg
          [ SvgAttrib.viewBox <| "0 0 " ++ width ++ " " ++ height ]
          items
      ]


type PointType = Start | End | Normal


viewPoints : (NodeId -> PointType) -> List (Positioned (Node ())) -> List (Svg a)
viewPoints pickType =
  List.sortBy .id >> List.map (\p -> viewPoint (pickType p.id) p)


viewPoint : PointType -> Positioned (Node ()) -> Svg a
viewPoint typ point =
  let
    size = { x = 50, y = 30 }

    start = point.pos .- (size ./ 2)

    color =
      case typ of
        Start ->
          "#ddf"
        
        End ->
          "#fdd"
        
        Normal ->
          "white"
  in
    Svg.g []
      [ Svg.rect
        [ SvgAttrib.x <| toString start.x
        , SvgAttrib.y <| toString start.y
        , SvgAttrib.width <| toString size.x
        , SvgAttrib.height <| toString size.y
        , SvgAttrib.fill color
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


viewEdge : Int -> Graph.Edge (Positioned (Node ())) EdgeData -> Svg a
viewEdge tick edge =
  let
    center = (edge.first.pos .+ edge.second.pos) ./ 2

    usedBefore =
      edge.data.lastTravel
      |> Maybe.map (\t -> tick - t)

    width =
      usedBefore
      |> Maybe.map (\b -> max 1 (6 - b // 2))
      |> Maybe.withDefault 1
  in
    Svg.g []
      [ Svg.line
        [ SvgAttrib.x1 <| toString edge.first.pos.x
        , SvgAttrib.y1 <| toString edge.first.pos.y
        , SvgAttrib.x2 <| toString edge.second.pos.x
        , SvgAttrib.y2 <| toString edge.second.pos.y
        , SvgAttrib.stroke "black"
        , SvgAttrib.strokeWidth <| toString width
        ]
        []
      , Svg.circle
        [ SvgAttrib.cx <| toString center.x
        , SvgAttrib.cy <| toString center.y
        , SvgAttrib.r "12"
        , SvgAttrib.fill "white"
        , SvgAttrib.stroke "black"
        , SvgAttrib.strokeWidth "2"
        ]
        []
      , Svg.text_
        [ SvgAttrib.x <| toString center.x
        , SvgAttrib.y <| toString (center.y + 1)
        , SvgAttrib.textAnchor "middle"
        , SvgAttrib.alignmentBaseline "middle"
        ]
        [ Svg.text <| toString edge.data.cost ]
      ]
