module NetworkCommon exposing (..)

import Visualised exposing (Visualised)


type alias NodeId = String


type alias Message a =
  { data : a
  , sender : NodeId
  , cost : Int
  }


type alias EdgeData =
  { cost : Int
  , lastTravel : Maybe Int
  }


type alias Node a =
  { id : NodeId
  , data : a
  }


type alias Network a = Visualised (Node a) EdgeData


type alias Simulation a b =
  { network : Network a
  , init : a
  , update : Message b -> a -> a
  , announce : a -> b
  , route : a -> NodeId -> Maybe NodeId
  , disconnect : NodeId -> a -> a
  }
