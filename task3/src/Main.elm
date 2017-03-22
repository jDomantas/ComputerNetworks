import Html

main : Program Never () ()
main = Html.program
    { init = ((), Cmd.none)
    , update = \ () () -> ((), Cmd.none)
    , view = \ () -> Html.text "Hello, world!"
    , subscriptions = \ () -> Sub.none
    }
