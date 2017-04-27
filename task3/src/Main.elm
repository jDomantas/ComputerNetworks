import Html exposing (Html)
import Html.Attributes as HtmlAttrib
import Window
import Task
import Time
import Random
import Dom
import Dom.Scroll
import Point exposing (Point)
import Network exposing (Sim)
import Terminal
import Command exposing (Command)


type alias Model =
  { width : Int
  , height : Int
  , tick : Int
  , tickProgress : Float
  , start : Maybe String
  , end : Maybe String
  , simulation : Sim
  , terminal : Terminal.Model
  }


type Msg
  = UpdateSize (Maybe Int) (Maybe Int)
  | RandomUpdate Int
  | Tick
  | NoOp
  | Terminal Terminal.Msg


main : Program Never Model Msg
main = Html.program
  { init = init
  , update = update
  , view = view
  , subscriptions = subscriptions
  }


init : (Model, Cmd Msg)
init =
  let
    terminal = writeHelp Terminal.init

    model =
      { width = 600
      , height = 100
      , tick = 0
      , tickProgress = 0
      , start = Nothing
      , end = Nothing
      , simulation = Network.distanceVector
      , terminal = terminal
      }

    tasks = Cmd.batch
      [ Task.perform (\w -> UpdateSize (Just w) Nothing) Window.width
      , Task.perform (\h -> UpdateSize Nothing (Just h)) Window.height
      , Task.attempt (always NoOp) (Dom.focus "terminput")
      ]
  in
    (model, tasks)


updateWithDom : Msg -> Model -> (Model, Cmd Msg)
updateWithDom msg model =
  let
    (newModel, cmds) = update msg model
  in
    (newModel, Cmd.batch [ scrollAndFocusTerminal, cmds ])


scrollAndFocusTerminal : Cmd Msg
scrollAndFocusTerminal =
  Cmd.batch
    [ Task.attempt (always NoOp) (Dom.focus "terminput")
    , Task.attempt (always NoOp) (Dom.Scroll.toBottom "terminal")
    ]


subscriptions : Model -> Sub Msg
subscriptions _ =
  Sub.batch
    [ Window.resizes (\size -> UpdateSize (Just size.width) (Just size.height))
    , Time.every (Time.second / 60.0) (always Tick)
    ]


writeHelp : Terminal.Model -> Terminal.Model
writeHelp terminal =
  terminal
  |> Terminal.write "Commands:"
  |> Terminal.write " help"
  |> Terminal.write "   shows this list"
  |> Terminal.write " add <node>"
  |> Terminal.write "   adds node named <node>"
  |> Terminal.write " remove <node>"
  |> Terminal.write "   removes node named <node>"
  |> Terminal.write " start <node>"
  |> Terminal.write "   sets start node to <node>"
  |> Terminal.write " end <node>"
  |> Terminal.write "   sets end node to <node>"
  |> Terminal.write " edge <a> <b>"
  |> Terminal.write "   removes edge between nodes <a> and <b>"
  |> Terminal.write " edge <a> <b> <weight>"
  |> Terminal.write "   adds edge between nodes <a> and <b>, with weight <weight>"
  |> Terminal.write " view <node>"
  |> Terminal.write "   shows routing table for <node>"


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
        , start = if Just node == model.start then Nothing else model.start
        , end = if Just node == model.end then Nothing else model.end
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

    Ok (Command.SetStart node) ->
      { model
        | start = Just node
        , terminal = Terminal.write ("New start node: " ++ node) model.terminal
        }

    Ok (Command.SetEnd node) ->
      { model
        | end = Just node
        , terminal = Terminal.write ("New end node: " ++ node) model.terminal
        }

    Ok (Command.ViewNode node) ->
      let
        data =
          Network.viewNode node model.simulation
          |> Maybe.withDefault [ "Node " ++ node ++ " does not exist" ]
        
        term =
          List.foldl (\line term -> Terminal.write line term) model.terminal data
      in
        { model | terminal = term }

    Ok Command.Help ->
      { model | terminal = writeHelp model.terminal }

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


update : Msg -> Model -> (Model, Cmd Msg)
update msg model =
  case msg of
    NoOp ->
      model ! []

    UpdateSize width height ->
      let
        newWidth = Maybe.withDefault model.width width
        newHeight = Maybe.withDefault model.height height
      in
        { model | width = newWidth, height = newHeight } ! []

    Tick ->
      let
        center =
          { x = toFloat (model.width - 400) / 2
          , y = toFloat model.height / 2
          }

        updatedSimulation =
          Network.animate (1 / 10.0) center model.simulation

        markRoute =
          if model.tick % 10 == 0 then
            Maybe.map3 Network.markRoute model.start model.end (Just model.tick)
            |> Maybe.withDefault identity
          else
            identity

        tickChange = if model.tickProgress >= 1 then 1 else 0

        commands =
          if model.tickProgress >= 1 then
            [ Random.generate RandomUpdate (Random.int 0 1000000000) ]
          else
            []
      in
        { model
          | simulation = markRoute updatedSimulation
          , tick = model.tick + tickChange
          , tickProgress = model.tickProgress - tickChange + 0.2
          } ! commands

    RandomUpdate value ->
      let
        updatedSimulation =
          Network.update model.tick model.simulation
      in
        { model | simulation = updatedSimulation } ! []

    Terminal msg ->
      let
        (terminal, command) = Terminal.update msg model.terminal

        runCommand =
          command
          |> Maybe.map applyCommand
          |> Maybe.withDefault identity
      in
        runCommand { model | terminal = terminal }
          ! [ Task.attempt (always NoOp) (Dom.Scroll.toBottom "terminal") ]


view : Model -> Html Msg
view model =
  let
    stylesheet =
      Html.node
        "link"
        [ HtmlAttrib.attribute "rel" "stylesheet"
        , HtmlAttrib.attribute "property" "stylesheet"
        , HtmlAttrib.attribute "href" "style.css"
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

    network = Network.view size model.start model.end model.simulation
  in
    Html.div []
      [ Html.map Terminal terminal
      , network
      ]
