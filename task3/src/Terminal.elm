module Terminal exposing
  ( Model, Msg
  , init, update, write, view
  )

import Regex
import Html exposing (Html)
import Html.Attributes as Attrib
import Html.Events as Events
import Json.Decode as Json


type alias Model =
  { input : String
  , output : List String
  }


type Msg
  = Input String
  | Enter


init : Model
init = Model "" []


update : Msg -> Model -> (Model, Maybe String)
update msg model =
  case msg of
    Input input ->
      ({ model | input = input }, Nothing)

    Enter ->
      ({ model | input = "" }, Just model.input)


write : String -> Model -> Model
write line model =
  let
    output =
      line :: model.output
      |> List.take 100
  in
    { model | output = output }


view : Model -> Html Msg
view model =
  Html.div
    [ Attrib.class "terminal"
    , Attrib.id "terminal"
    ]
    [ viewOutput model.output
    , viewInput model.input
    ]


viewOutput : List String -> Html a
viewOutput items =
  let
    fixSpaces =
      Regex.replace Regex.All (Regex.regex " ") (always "\x00A0")

    viewLine line =
      Html.p [] [ Html.text (fixSpaces line) ]
  in
    items
    |> List.reverse
    |> List.map viewLine
    |> Html.div []


viewInput : String -> Html Msg
viewInput input =
  Html.input
    [ Attrib.type_ "text"
    , Attrib.value input
    , Attrib.id "terminput"
    , Events.onInput Input
    , onEnter Enter
    ]
    []


onEnter : Msg -> Html.Attribute Msg
onEnter msg =
  let
    isEnter code =
      if code == 13 then
        Json.succeed msg
      else
        Json.fail ""
  in
    Events.on "keydown" (Json.andThen isEnter Events.keyCode)
