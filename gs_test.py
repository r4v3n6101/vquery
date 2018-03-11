from vquery import *

server = ValveQuery('46.174.48.49', 27201, SOURCE, 1)
challenge = server.get_challenge()

print(server.a2s_info())
print(server.a2s_player(challenge))
print(server.a2s_rules(challenge))
