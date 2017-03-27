import Test exposing (Test)
import Test.Runner.Html
import Tests.DistanceVector


main : Test.Runner.Html.TestProgram
main =
  suites
  |> Test.concat
  |> Test.Runner.Html.run


suites : List Test
suites =
  [ Tests.DistanceVector.suite
  ]
