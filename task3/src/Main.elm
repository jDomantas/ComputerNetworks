import Html exposing (Html)
import Html.Attributes as HtmlAttrib
import Window
import Task
import Time
import Point exposing (Point)
import Network exposing (Sim)
import Terminal
import Command exposing (Command)


type alias Model =
  { width : Int
  , height : Int
  , simulation : Sim
  , terminal : Terminal.Model
  }


type Msg
  = UpdateSize (Maybe Int) (Maybe Int)
  | Tick
  | Terminal Terminal.Msg


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
      { width = 600
      , height = 100
      , simulation = Network.distanceVector
      , terminal = Terminal.init
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
    [ Window.resizes (\size -> UpdateSize (Just size.width) (Just size.height))
    , Time.every (Time.second / 60.0) (always Tick)
    ]


applyCommand : String -> Model -> Model
applyCommand cmd model =
  case Command.parse cmd of
    Ok (Command.AddNode node) ->
      { model
        | simulation = Network.addNode node model.simulation
        , terminal = Terminal.write ("Added node " ++ node) model.terminal
        }

    Ok (Command.RemoveNode node) ->
      { model
        | simulation = Network.removeNode node model.simulation
        , terminal = Terminal.write ("Removed node " ++ node) model.terminal
        }

    Ok (Command.UpdateEdge start end cost) ->
      let
        msg =
          case cost of
            Just cost ->
              "Added edge between " ++ start ++ " and " ++ end ++ " with cost " ++ toString cost

            Nothing ->
              "Removed edge between " ++ start ++ " and " ++ end
      in
        { model
          | simulation = Network.updateEdge start end cost model.simulation
          , terminal = Terminal.write msg model.terminal
          }

    Ok cmd ->
      let
        msg = "Command not implemented: " ++ toString cmd
      in
        { model | terminal = Terminal.write msg model.terminal }
    
    Err error ->
      let
        term =
          model.terminal
          |> Terminal.write cmd
          |> Terminal.write "bad command"
      in
        { model | terminal = term }


update : Msg -> Model -> Model
update msg model =
  case msg of
    UpdateSize width height ->
      let
        newWidth = Maybe.withDefault model.width width
        newHeight = Maybe.withDefault model.height height
      in
        { model | width = newWidth, height = newHeight }

    Tick ->
      let
        center =
          { x = toFloat (model.width - 400) / 2
          , y = toFloat model.height / 2
          }

        updatedSimulation =
          Network.animate (1 / 10.0) center model.simulation
      in
        { model | simulation = updatedSimulation }

    Terminal msg ->
      let
        (terminal, command) = Terminal.update msg model.terminal

        runCommand =
          command
          |> Maybe.map applyCommand
          |> Maybe.withDefault identity
      in
        runCommand { model | terminal = terminal }


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
    size = Point (toFloat (model.width - 400)) (toFloat model.height)

    terminal = Terminal.view model.terminal

    network = Network.view size model.simulation
  in
    Html.div []
      [ Html.map Terminal terminal
      , network
      ]
