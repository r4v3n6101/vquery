import io
import socket
import struct


def build_packet(packet_type):
    return struct.pack('<lB', -1, packet_type)


def build_packet_challenge(packet_type, challenge):
    return build_packet(packet_type) + struct.pack('<i', challenge)


A2S_INFO_PACKET = build_packet(ord('T')) + b'Source Engine Query\x00'
A2S_PLAYER_CHALLENGE_PACKET = build_packet_challenge(ord('U'), -1)
A2S_RULES_CHALLENGE_PACKET = build_packet_challenge(ord('V'), -1)
PACKET_SIZE = 1400


class Buffer(io.BytesIO):

    def read_string(self):
        val = self.getvalue()
        start = self.tell()
        end = val.index(b'\0', start)
        self.seek(end + 1)
        return val[start:end].decode('utf-8')

    def read_float(self):
        return struct.unpack("<f", self.read(4))[0]

    def read_int(self):
        return struct.unpack("<l", self.read(4))[0]

    def read_byte(self):
        return struct.unpack("<B", self.read(1))[0]

    def read_short(self):
        return struct.unpack('<h', self.read(2))[0]

    def write_byte(self, byte):
        self.write(struct.pack('<B', byte))

    def write_int(self, integer):
        self.write(struct.pack('<l', integer))


class GoldsrcQuery:

    def __init__(self, host, port, **kwargs):
        self.socket = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        if 'timeout' in kwargs:
            self.socket.settimeout(kwargs['timeout'])
        self.socket.connect((host, port))

    def read(self):
        packet = Buffer(self.socket.recv(PACKET_SIZE))
        header = packet.read_int()
        if header == -1:
            return packet
        else:
            ident = packet.read_int()
            num = packet.read_byte()
            packets_num = num & 0x0F
            packets = [0] * packets_num
            index = (num & 0xF0) >> 4

            packets[index] = packet.read()
            while 0 in packets:
                packet = Buffer(self.socket.recv(PACKET_SIZE))
                header = packet.read_int()
                ident = packet.read_int()
                num = packet.read_byte()
                index = (num & 0xF0) >> 4
                packets[index] = packet.read()
            return Buffer(b''.join(packets))

    def a2s_info(self):
        self.socket.send(A2S_INFO_PACKET)
        response = self.read()
        response.read_byte()  # TODO: Header for checking version of response
        result = {
            'address': response.read_string(),
            'name': response.read_string(),
            'map': response.read_string(),
            'folder': response.read_string(),
            'game': response.read_string(),
            'players': response.read_byte(),
            'max_players': response.read_byte(),
            'protocol': response.read_byte(),
            'server_type': chr(response.read_byte()),
            'enviroment': chr(response.read_byte()),
            'visibility': bool(response.read_byte())
        }
        if response.read_byte() == 1:
            mod = {'link': response.read_string(), 'download_link': response.read_string()}
            response.read_byte()  # Skip NUL byte
            mod['version'] = response.read_int()
            mod['size'] = response.read_int()
            mod['mod_type'] = response.read_byte()
            mod['dll'] = response.read_byte()
            result['mod'] = mod

        result['vac'] = bool(response.read_byte())
        result['bots'] = response.read_byte()
        return result

    def a2s_players_challenge(self):
        self.socket.send(A2S_PLAYER_CHALLENGE_PACKET)
        response = self.read()
        response.read_byte()  # TODO : Check header
        return response.read_int()

    def a2s_players(self, challenge):
        self.socket.send(build_packet_challenge(ord('U'), challenge))
        response = self.read()
        response.read_byte()  # TODO : Header
        players_num = response.read_byte()
        players = []
        for i in range(players_num):
            players.append({
                'index': response.read_byte(),
                'name': response.read_string(),
                'score': response.read_int(),
                'duration': response.read_float()
            })
        return {
            'players_num': players_num,
            'players': players
        }

    def a2s_rules_challenge(self):
        self.socket.send(A2S_RULES_CHALLENGE_PACKET)
        response = self.read()
        response.read_byte()
        return response.read_int()

    def a2s_rules(self, challenge):
        self.socket.send(build_packet_challenge(ord('V'), challenge))
        response = self.read()
        if response.read_int() != -1:  # 4 FF bytes ain't required
            response.seek(response.tell() - 4)  # ... so we're gonna back to unreaded data

        header = response.read_byte()
        size = response.read_short()
        rules = {}
        for i in range(0, size, 2):
            key = response.read_string()
            value = response.read_string()
            rules[key] = value
        return {
            'size': size,
            'rules': rules
        }
