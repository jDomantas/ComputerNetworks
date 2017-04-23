module DistanceVector exposing
  ( Sim, Model, Msg
  , init, update, announce, route, disconnect, view
  )

import NetworkCommon exposing (NodeId, Message, Simulation)
import List.Extra


type alias TableEntry =
  { id : NodeId
  , cost : Int
  , hop : NodeId
  }


type alias Model =
  { myId : NodeId
  , table : List TableEntry
  }


type alias MsgEntry =
  { id : NodeId
  , cost : Int
  }


type alias Msg = List MsgEntry


type alias Sim = Simulation Model Msg


-- at the start routing table is empty
init : NodeId -> Model
init id =
  { myId = id
  , table = []
  }


-- updating:
--   clear all previous routes that go through sender
--   add new routes going through sender
--   for each destination, keep only best route
update : Message Msg -> Model -> Model
update msg model =
  let
    validEntry entry =
      entry.id /= model.myId
      && entry.hop /= model.myId
      && entry.cost < 100

    table =
      model.table
      |> List.filter (\entry -> entry.hop /= msg.sender)
      |> List.append (List.map (\entry ->
        { id = entry.id
        , cost = msg.cost + entry.cost
        , hop = msg.sender
        }) msg.data)
      -- add direct route to sender
      |> (::) { id = msg.sender, cost = msg.cost, hop = msg.sender }
      |> List.sortBy .id
      |> List.Extra.groupWhile (\a b -> a.id == b.id)
      |> List.map (List.sortBy .cost)
      |> List.filterMap List.head
      |> List.filter validEntry
  in
    { model | table = table }
  

-- to announce send all table to neighbour
announce : Model -> Msg
announce =
  .table >> List.map (\entry -> MsgEntry entry.id entry.cost)


-- routing is just looking up that destination in routing table
route : Model -> NodeId -> Maybe NodeId
route model id =
  model.table
  |> List.Extra.find (\entry -> entry.id == id)
  |> Maybe.map .hop


-- when neighbour node is disconnected, remove
-- all paths that are known to go through it
disconnect : NodeId -> Model -> Model
disconnect id model =
  { model | table = List.filter (\entry -> entry.hop /= id) model.table }


view : Model -> List String
view model =
  let
    formatRow destination hop cost =
      "| " ++ (String.pad 4 ' ' destination) ++
      " | " ++ (Debug.log "padded" <| String.pad 4 ' ' hop) ++
      " | " ++ (String.pad 4 ' ' cost) ++ " |"
    
    formatEntry entry =
      formatRow entry.id entry.hop (toString entry.cost)
  in
    [ "| Node " ++ model.myId
    , formatRow "dest" "hop" "cost"
    , "+------+------+------+"
    ] ++ List.map formatEntry model.table
