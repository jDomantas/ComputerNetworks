module Point exposing (..)


type alias Point =
  { x : Float
  , y : Float
  }


(.+) : Point -> Point -> Point
(.+) a b = { x = a.x + b.x, y = a.y + b.y }


(.-) : Point -> Point -> Point
(.-) a b = { x = a.x - b.x, y = a.y - b.y }


(.*) : Point -> Float -> Point
(.*) a b = { x = a.x * b, y = a.y * b }


(./) : Point -> Float -> Point
(./) a b = { x = a.x / b, y = a.y / b }


lengthSquared : Point -> Float
lengthSquared { x, y } = x * x + y * y


length : Point -> Float
length { x, y} = sqrt (x * x + y * y)
