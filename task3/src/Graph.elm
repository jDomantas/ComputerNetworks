module Graph exposing
  ( Graph, Edge
  , empty, addNode, connect
  , findNode, findEdge
  , nodes, edges, neighbours
  , mapNodes, mapEdges, mapFullEdges
  , filterNodes, filterEdges, filterFullEdges
  , anyNode, anyEdge
  )

import List.Extra


type alias RawNode n =
  { id : Int
  , data : n
  }


type alias RawEdge e =
  { first : Int
  , second : Int
  , data: e
  }


type alias Edge n e =
  { first : n
  , second : n
  , data : e
  }


type Graph n e =
  Graph
    { nodes : List (RawNode n)
    , edges : List (RawEdge e)
    }


empty : Graph n e
empty =
  Graph
    { nodes = []
    , edges = []
    }


addNode : n -> Graph n e -> Graph n e
addNode data (Graph graph) =
  let
    newId =
      graph.nodes
      |> List.map .id
      |> List.maximum
      |> Maybe.withDefault 0
      |> (+) 1

    node =
      { id = newId
      , data = data
      }
  in
    Graph { graph | nodes = node :: graph.nodes }


connect : (n -> Bool) -> (n -> Bool) -> e -> Graph n e -> Graph n e
connect pred1 pred2 data (Graph graph) =
  case (findNodeId pred1 (Graph graph), findNodeId pred2 (Graph graph)) of
    (Just id1, Just id2) ->
      let
        newEdge =
          { first = id1
          , second = id2
          , data = data
          }
      in
        if canAddEdge id1 id2 (Graph graph) then
          Graph { graph | edges = newEdge :: graph.edges }
        else
          Graph graph

    _ ->
      Graph graph


findNode : (n -> Bool) -> Graph n e -> Maybe n
findNode predicate (Graph graph) =
  graph.nodes
  |> List.Extra.find (predicate << .data)
  |> Maybe.map .data


findEdge : (e -> Bool) -> Graph n e -> Maybe e
findEdge predicate (Graph graph) =
  graph.edges
  |> List.Extra.find (predicate << .data)
  |> Maybe.map .data


nodes : Graph n e -> List n
nodes (Graph graph) =
  List.map .data graph.nodes


edges : Graph n e -> List (Edge n e)
edges (Graph graph) =
  graph.edges
  |> List.map (\edge ->
    let
      first = unsafeGetNode edge.first (Graph graph)
      second = unsafeGetNode edge.second (Graph graph)
    in
      { first = first
      , second = second
      , data = edge.data
      })


neighbours : (n -> Bool) -> Graph n e -> Maybe (List (n, e))
neighbours predicate graph =
  rawNeighbours predicate graph
  |> Maybe.map (\edges ->
    List.map (\edge -> (unsafeGetNode edge.second graph, edge.data)) edges)


mapNodes : (n -> t) -> Graph n e -> Graph t e
mapNodes f (Graph graph) =
  let
    nodes = List.map (\node -> { node | data = f node.data }) graph.nodes
  in
    Graph { graph | nodes = nodes }


mapEdges : (e -> t) -> Graph n e -> Graph n t
mapEdges f (Graph graph) =
  let
    edges = List.map (\edge -> { edge | data = f edge.data }) graph.edges
  in
    Graph { graph | edges = edges }


mapFullEdges : (Edge n e -> t) -> Graph n e -> Graph n t
mapFullEdges f (Graph graph) =
  let
    fullEdge edge =
      let
      first = unsafeGetNode edge.first (Graph graph)
      second = unsafeGetNode edge.second (Graph graph)
    in
      { first = first
      , second = second
      , data = edge.data
      }

    mapper edge =
      let
        data = f (fullEdge edge)
      in
        { edge | data = data }

    edges = List.map mapper graph.edges
  in
    Graph { graph | edges = edges }

filterNodes : (n -> Bool) -> Graph n e -> Graph n e
filterNodes predicate (Graph graph) =
  let
    nodes = List.filter (predicate << .data) graph.nodes

    hasNode id = List.any (\node -> node.id == id) nodes

    edges = List.filter (\edge -> hasNode edge.first && hasNode edge.second) graph.edges
  in
    Graph
      { nodes = nodes
      , edges = edges
      }


filterEdges : (e -> Bool) -> Graph n e -> Graph n e
filterEdges predicate (Graph graph) =
  let
    edges = List.filter (predicate << .data) graph.edges
  in
    Graph { graph | edges = edges }


filterFullEdges : (Edge n e -> Bool) -> Graph n e -> Graph n e
filterFullEdges predicate (Graph graph) =
  let
    fullEdge edge =
      let
      first = unsafeGetNode edge.first (Graph graph)
      second = unsafeGetNode edge.second (Graph graph)
    in
      { first = first
      , second = second
      , data = edge.data
      }

    edges = List.filter (predicate << fullEdge) graph.edges
  in
    Graph { graph | edges = edges }


anyNode : (n -> Bool) -> Graph n e -> Bool
anyNode predicate (Graph graph) =
  List.any (predicate << .data) graph.nodes


anyEdge : (e -> Bool) -> Graph n e -> Bool
anyEdge predicate (Graph graph) =
  List.any (predicate << .data) graph.edges


unsafeGetNode : Int -> Graph n e -> n
unsafeGetNode id (Graph graph) =
  case List.Extra.find (\node -> node.id == id) graph.nodes of
    Just node ->
      node.data

    Nothing ->
      Debug.crash "node is referenced by an edge, but it is not in the graph"


findNodeId : (n -> Bool) -> Graph n e -> Maybe Int
findNodeId predicate (Graph graph) =
  graph.nodes
  |> List.Extra.find (predicate << .data)
  |> Maybe.map .id


rawNeighbours : (n -> Bool) -> Graph n e -> Maybe (List (RawEdge e))
rawNeighbours predicate (Graph graph) =
  findNodeId predicate (Graph graph)
  |> Maybe.map (\id ->
    graph.edges
    |> List.filterMap (\edge ->
      if edge.first == id then
        Just edge
      else if edge.second == id then
        Just { edge | first = edge.second, second = edge.first }
      else
        Nothing))


canAddEdge : Int -> Int -> Graph n e -> Bool
canAddEdge a b (Graph graph) =
  let
    sameEdge edge =
      (edge.first == a && edge.second == b) ||
        (edge.second == a && edge.first == b)
  in
    if a == b then
      False
    else if List.any sameEdge graph.edges then
      False
    else
      True
