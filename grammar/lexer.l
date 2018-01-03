%%

\[  LSQUARE
\]  RSQUARE
\.  DOT
;   SEMI
,   COMMA
\{  LBRACE
\}  RBRACE
=   EQ
-   MINUS
\+  PLUS
==  EQEQ
<=  LTEQ
>=  GTEQ
<   LT
>   GT
\(  LPAREN
\)  RPAREN

if      IF
else    ELSE
def     DEF
class   CLASS
return  RETURN
print   PRINT
let     LET
for     FOR
new     NEW
this    THIS
try     TRY
except  EXCEPT
raise   RAISE

[0-9]+                  INT_LITERAL
(true|false)            BOOL_LITERAL
"(?:\\"|[^"\n])*"       STR_LITERAL
[ \t\n\r]+              ;
(//.*?$|/[*].*?[*]/)    ;
[a-zA-Z_][a-zA-Z0-9_]*  IDENTIFIER
