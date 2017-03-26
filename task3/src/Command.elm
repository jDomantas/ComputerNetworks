module Command exposing
  ( Command(..)
  , parse
  )

import Parser exposing (Parser, (|.), (|=))


type Command
  = AddNode String
  | RemoveNode String
  | UpdateEdge String String (Maybe Int)
  | SetStart String
  | SetEnd String
  | ViewNode String
  | Clear


parse : String -> Result String Command
parse input =
  case Parser.run parser input of
    Ok command ->
      Ok command

    Err error ->
      Err (toString error.problem)


parser : Parser Command
parser =
  Parser.succeed identity
  |. maybeSpaces
  |= command
  |. maybeSpaces


command : Parser Command
command =
  Parser.oneOf
    [ addNode
    , removeNode
    , updateEdge
    , setStart
    , setEnd
    , viewNode
    , clear
    ]


withNameArg : String -> (String -> a) -> Parser a
withNameArg command f =
  Parser.succeed f
  |. Parser.symbol command
  |. spaces
  |= name


addNode : Parser Command
addNode = withNameArg "add" AddNode


removeNode : Parser Command
removeNode = withNameArg "remove" RemoveNode


setStart : Parser Command
setStart = withNameArg "start" SetStart


setEnd : Parser Command
setEnd = withNameArg "end" SetEnd


viewNode : Parser Command
viewNode = withNameArg "view" ViewNode


updateEdge : Parser Command
updateEdge =
  Parser.succeed UpdateEdge
  |. Parser.symbol "edge"
  |. spaces
  |= name
  |. spaces
  |= name
  |= (Parser.oneOf
    [ Parser.succeed Just
      |. spaces
      |= number
    , Parser.succeed Nothing
    ])


clear : Parser Command
clear =
  Parser.succeed Clear
  |. Parser.symbol "clear"


spaces : Parser ()
spaces =
  Parser.ignore Parser.oneOrMore (\c -> c == ' ')


maybeSpaces : Parser ()
maybeSpaces =
  Parser.ignore Parser.zeroOrMore (\c -> c == ' ')


number : Parser Int
number =
  Parser.int


letters : String
letters = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"


name : Parser String
name =
  Parser.keep Parser.oneOrMore (\c -> String.contains (String.fromChar c) letters)
