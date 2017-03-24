import Html exposing (Html)
import Html.Attributes as HtmlAttrib
import Mouse
import Window
import Task
import Time
import Svg exposing (Svg)
import Svg.Attributes as SvgAttrib
import Graph exposing (Graph)
import Visualised exposing (Visualised, Positioned)
import Point exposing (Point)
import Network


type alias Node =
  { id : Int
  }


type alias Model =
  { width : Int
  , height : Int
  , graph : Visualised Node ()
  }


type Msg
  = AddPoint Point
  | UpdateSize (Maybe Int) (Maybe Int)
  | Tick


main : Program Never Model Msg
main = Html.program
  { init = init
  , update = \msg model -> (update msg model, Cmd.none)
  , view = view
  , subscriptions = subscriptions
  }


init : (Model, Cmd Msg)
init =
  let
    model =
      { width = 100
      , height = 100
      , graph = Graph.empty
      }

    tasks = Cmd.batch
      [ Task.perform (\w -> UpdateSize (Just w) Nothing) Window.width
      , Task.perform (\h -> UpdateSize Nothing (Just h)) Window.height
      ]
  in
    (model, tasks)


subscriptions : Model -> Sub Msg
subscriptions _ =
  Sub.batch
    [ Mouse.clicks (\pos -> AddPoint { x = toFloat pos.x, y = toFloat pos.y })
    , Window.resizes (\size -> UpdateSize (Just size.width) (Just size.height))
    , Time.every (Time.second / 60.0) (always Tick)
    ]


gcd : Int -> Int -> Int
gcd a b =
  if b == 0 then
    a
  else
    gcd b (a % b)


connectId : Int -> Maybe Int
connectId id =
  List.range 1 (id - 1)
  |> List.map (\n -> (gcd id n, n))
  |> List.maximum
  |> Maybe.map (\(_, n) -> n)


update : Msg -> Model -> Model
update msg model =
  case msg of
    AddPoint pt ->
      let
        newId =
          model.graph
          |> Graph.nodes
          |> List.map .id
          |> List.maximum
          |> Maybe.withDefault 0
          |> (+) 1
        
        newNode =
          { pos = pt
          , v = Point.zero
          , a = Point.zero
          , id = newId
          }

        graph =
          model.graph
          |> Graph.addNode newNode
          |> Graph.connect (\n -> n.id == newNode.id) (\n -> Just n.id == connectId newNode.id) ()
      in
        { model | graph = graph }
    
    UpdateSize width height ->
      let
        newWidth = Maybe.withDefault model.width width
        newHeight = Maybe.withDefault model.height height
      in
        { model | width = newWidth, height = newHeight }

    Tick ->
      let
        center =
          { x = toFloat model.width / 2
          , y = toFloat model.height / 2
          }

        updatedGraph =
          Visualised.simulate (1 / 10.0) center model.graph
      in
        { model | graph = updatedGraph }


view : Model -> Html a
view model =
  let
    stylesheet =
      Html.node
        "link"
        [ HtmlAttrib.attribute "rel" "stylesheet"
        , HtmlAttrib.attribute "property" "stylesheet"
        , HtmlAttrib.attribute "href" "../style.css"
        ]
        []
  in
    Html.div []
      [ stylesheet
      , viewModel model
      ]


viewModel : Model -> Html a
viewModel model =
  let
    width = toString model.width

    height = toString model.height

    text =
      Svg.text_
        [ SvgAttrib.alignmentBaseline "hanging" ]
        [ Svg.text <| "width: " ++ width ++ ", height: " ++ height ]

    points = viewPoints (Graph.nodes model.graph)

    edges =
      model.graph
      |> Graph.edges
      |> List.map viewEdge

    items = edges ++ points ++ [ text ]
  in
    Svg.svg
      [ SvgAttrib.viewBox <| "0 0 " ++ width ++ " " ++ height
      ]
      items


viewPoints : List (Positioned Node) -> List (Svg a)
viewPoints =
  List.sortBy .id >> List.map viewPoint


viewPoint : Positioned Node -> Svg a
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
      [ Svg.text <| toString point.id ]
    ]


viewEdge : { first : Positioned Node, second : Positioned Node, data : () } -> Svg a
viewEdge edge =
  Svg.line
    [ SvgAttrib.x1 <| toString edge.first.pos.x
    , SvgAttrib.y1 <| toString edge.first.pos.y
    , SvgAttrib.x2 <| toString edge.second.pos.x
    , SvgAttrib.y2 <| toString edge.second.pos.y
    , SvgAttrib.stroke "black"
    ]
    []
