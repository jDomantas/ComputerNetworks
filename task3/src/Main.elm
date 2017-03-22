import Html exposing (Html)
import Html.Attributes as HtmlAttrib
import Mouse
import Window
import Task
import Svg exposing (Svg)
import Svg.Attributes as SvgAttrib
import Graph exposing (Graph)


type alias Point =
  { x : Int
  , y : Int
  }


type alias Node =
  { position : Point
  , id : Int
  }


type alias Model =
  { width : Int
  , height : Int
  , graph : Graph Node ()
  }


type Msg
  = AddPoint Point
  | UpdateSize (Maybe Int) (Maybe Int)


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
    [ Mouse.clicks AddPoint
    , Window.resizes (\size -> UpdateSize (Just size.width) (Just size.height))
    ]


gcd : Int -> Int -> Int
gcd a b =
  if b == 0 then
    a
  else
    gcd b (a % b)


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
          { position = pt
          , id = newId
          }

        graph =
          model.graph
          |> Graph.addNode newNode
          |> Graph.connect (\n -> n.id == newNode.id) (\n -> n.id /= newNode.id && gcd n.id newNode.id /= 1) ()
      in
        { model | graph = graph }
    
    UpdateSize width height ->
      let
        newWidth = Maybe.withDefault model.width width
        newHeight = Maybe.withDefault model.height height
      in
        { model | width = newWidth, height = newHeight }


view : Model -> Html Msg
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


viewModel : Model -> Html Msg
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
    Html.map never <| Svg.svg
      [ SvgAttrib.viewBox <| "0 0 " ++ width ++ " " ++ height
      ]
      items


viewPoints : List Node ->List (Svg Never)
viewPoints =
  List.sortBy .id >> List.map viewPoint


viewPoint : Node -> Svg Never
viewPoint point =
  Svg.g []
    [ Svg.circle
      [ SvgAttrib.cx <| toString point.position.x
      , SvgAttrib.cy <| toString point.position.y
      , SvgAttrib.r "15"
      , SvgAttrib.fill "white"
      , SvgAttrib.stroke "black"
      , SvgAttrib.strokeWidth "2"
      ]
      []
    , Svg.text_
      [ SvgAttrib.x <| toString point.position.x
      , SvgAttrib.y <| toString point.position.y
      , SvgAttrib.textAnchor "middle"
      , SvgAttrib.alignmentBaseline "middle"
      ]
      [ Svg.text <| toString point.id ]
    ]


viewEdge : { first : Node, second : Node, data : () } -> Svg Never
viewEdge edge =
  Svg.line
    [ SvgAttrib.x1 <| toString edge.first.position.x
    , SvgAttrib.y1 <| toString edge.first.position.y
    , SvgAttrib.x2 <| toString edge.second.position.x
    , SvgAttrib.y2 <| toString edge.second.position.y
    , SvgAttrib.stroke "black"
    ]
    []
