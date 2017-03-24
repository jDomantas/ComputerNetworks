module DistanceVector exposing
  ( Sim, Table, Msg
  , init, update, announce, route, disconnect
  )

import NetworkCommon exposing (NodeId, Message, Simulation)
import List.Extra


type alias TableEntry =
  { id : NodeId
  , cost : Int
  , hop : NodeId
  }

type alias Table = List TableEntry


type alias MsgEntry =
  { id : NodeId
  , cost : Int
  }


type alias Msg = List MsgEntry


type alias Sim = Simulation Table Msg


-- at the start routing table is empty
init : Table
init = []


-- updating:
--   clear all previous routes that go through sender
--   add new routes going through sender
--   for each destination, keep only best route
update : Message Msg -> Table -> Table
update msg table =
  table
  |> List.filter (\entry -> entry.hop /= msg.sender)
  |> List.append (List.map (\entry ->
    { id = entry.id
    , cost = msg.cost + entry.cost
    , hop = msg.sender
    }) msg.data)
  |> List.Extra.groupWhile (\a b -> a.id == b.id)
  |> List.map (List.sortBy .cost)
  |> List.filterMap List.head
  

-- to announce send all table to neighbour
announce : Table -> Msg
announce =
  List.map (\entry -> MsgEntry entry.id entry.cost)


-- routing is just looking up that destination in routing table
route : Table -> NodeId -> Maybe NodeId
route table id =
  table
  |> List.Extra.find (\entry -> entry.id == id)
  |> Maybe.map .hop


-- when neighbour node is disconnected, remove
-- all paths that are known to go through it
disconnect : NodeId -> Table -> Table
disconnect id =
  List.filter (\entry -> entry.hop /= id)
