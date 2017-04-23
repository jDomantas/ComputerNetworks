module Tests.DistanceVector exposing (suite)

import Expect
import Test exposing (..)
import DistanceVector exposing (..)


suite : Test
suite =
  describe "distance vector"
    [ testInitialTable
    , testAnnounce
    , testDisconnectDirect
    , testDisconnectIndirect
    , describe "update"
      [ testUpdateAddsDirect
      , testUpdateUpdatesDirect
      , testUpdateImprovesIndirect
      , testUpdateCalculatedCost
      , testUpdateKeepsBest
      ]
    ]


testInitialTable : Test
testInitialTable = test "initial table is empty" <| \() ->
  (init "id").table |> Expect.equal []


testAnnounce : Test
testAnnounce = test "announcement contains the table" <| \() ->
  let
    model =
      { myId = "id"
      , table =
        [ { id = "a", hop = "a", cost = 1 }
        , { id = "b", hop = "b", cost = 3 }
        , { id = "c", hop = "a", cost = 7 }
        ]
      }
    
    expectedMessage =
      [ { id = "a", cost = 1 }
      , { id = "b", cost = 3 }
      , { id = "c", cost = 7 }
      ]
  in
    announce model |> Expect.equal expectedMessage


testDisconnectDirect : Test
testDisconnectDirect = test "disconnecting neighbour removes routes going through it" <| \() ->
  let
    model =
      { myId = "id"
      , table =
        [ { id = "a", hop = "a", cost = 1 }
        , { id = "b", hop = "c", cost = 1 }
        ]
      }
    
    expected =
      { myId = "id"
      , table =
        [ { id = "b", hop = "c", cost = 1 }
        ]
      }
  in
    disconnect "a" model |> Expect.equal expected


testDisconnectIndirect : Test
testDisconnectIndirect = test "disconnecting does not remove indirect routes" <| \() ->
  let
    model =
      { myId = "id"
      , table =
        [ { id = "a", hop = "b", cost = 1 }
        ]
      }
  in
    disconnect "a" model |> Expect.equal model


testUpdateAddsDirect : Test
testUpdateAddsDirect =
  test "sender is added as direct route" <| \() ->
    let
      model =
        { myId = "id"
        , table = []
        }
      
      msg = { sender = "a", cost = 5, data = [] }
      
      expected =
        { myId = "id"
        , table = [ { id = "a", hop = "a", cost = 5 } ]
        }
    in
      update msg model |> Expect.equal expected


testUpdateUpdatesDirect : Test
testUpdateUpdatesDirect =
  test "message always overrides direct routes" <| \() ->
    let
      model =
        { myId = "id"
        , table = [ { id = "a", hop = "a", cost = 3 } ]
        }
      
      msg = { sender = "a", cost = 5, data = [] }
      
      expected =
        { myId = "id"
        , table = [ { id = "a", hop = "a", cost = 5 } ]
        }
    in
      update msg model |> Expect.equal expected


testUpdateImprovesIndirect : Test
testUpdateImprovesIndirect =
  [ test "message improves worse indirect routes" <| \() ->
    let
      model =
        { myId = "id"
        , table = [ { id = "a", hop = "c", cost = 8 } ]
        }
      
      msg = { sender = "a", cost = 5, data = [] }
      
      expected =
        { myId = "id"
        , table = [ { id = "a", hop = "a", cost = 5 } ]
        }
    in
      update msg model |> Expect.equal expected
  , test "message does not improve better indirect routes" <| \() ->
    let
      model =
        { myId = "id"
        , table = [ { id = "a", hop = "c", cost = 3 } ]
        }
      
      msg = { sender = "a", cost = 5, data = [] }
      
      expected =
        { myId = "id"
        , table = [ { id = "a", hop = "c", cost = 3 } ]
        }
    in
      update msg model |> Expect.equal expected
  ]
  |> concat


testUpdateCalculatedCost : Test
testUpdateCalculatedCost =
  test "cost is sum of sending cost and lister cost" <| \() ->
    let
      model =
        { myId = "id"
        , table = []
        }
      
      msg = { sender = "a", cost = 5, data = [ { id = "b", cost = 3 } ] }
      
      expected =
        { myId = "id"
        , table =
          [ { id = "a", hop = "a", cost = 5 }
          , { id = "b", hop = "a", cost = 8 }
          ]
        }
    in
      update msg model |> Expect.equal expected


testUpdateKeepsBest : Test
testUpdateKeepsBest =
  test "only best route is kept" <| \() ->
    let
      model =
        { myId = "id"
        , table =
          [ { id = "b", hop = "c", cost = 3 }
          , { id = "d", hop = "e", cost = 10 }
          ]
        }
      
      msg =
        { sender = "a"
        , cost = 3
        , data =
          [ { id = "b", cost = 10 }
          , { id = "d", cost = 3 }
          ]
        }
      
      expected =
        { myId = "id"
        , table =
          [ { id = "a", hop = "a", cost = 3 }
          , { id = "b", hop = "c", cost = 3 }
          , { id = "d", hop = "a", cost = 6 }
          ]
        }
    in
      update msg model |> Expect.equal expected
