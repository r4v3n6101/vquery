"""
Utility for querying data from servers ran on goldsrc engine.
It doesn't support Source engine at all, because multiple packets aren't decompressed with bz2.
Also, utility doesn't support games like 'The Ship' that modified their network code.
Query docs: https://developer.valvesoftware.com/wiki/Server_queries
"""

import io
import socket
import struct

PACKET_SIZE = 1400
SINGLE = -1


def build_packet(packet_type):
    return struct.pack('<lB', SINGLE, packet_type)


def build_packet_challenge(packet_type, challenge):
    return build_packet(packet_type) + struct.pack('<i', challenge)


A2S_INFO_PACKET = build_packet(ord('T')) + b'Source Engine Query\0'
CHALLENGE_PACKET = build_packet_challenge(ord('U'), -1)


class Buffer(io.BytesIO):

    def read_string(self):
        val = self.getvalue()
        start = self.tell()
        end = val.index(b'\0', start)
        self.seek(end + 1)
        return val[start:end].decode('utf-8')

    def read_float(self):
        return struct.unpack('<f', self.read(4))[0]

    def read_int(self):
        return struct.unpack('<l', self.read(4))[0]

    def read_byte(self):
        return struct.unpack('<B', self.read(1))[0]

    def read_short(self):
        return struct.unpack('<h', self.read(2))[0]

    def read_long_long(self):
        return struct.unpack('<Q', self.read(8))[0]


class PacketError(Exception):
    pass


class GoldsrcQuery:

    def __init__(self, host, port, timeout=10.0):
        self.socket = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        self.socket.settimeout(timeout)
        self.socket.connect((host, port))

    # TODO : Source support
    def read(self):
        packet = Buffer(self.socket.recv(PACKET_SIZE))
        header = packet.read_int()
        if header == SINGLE:
            return packet
        else:
            packet_id = packet.read_int()
            num = packet.read_byte()
            packets_num = num & 0x0F
            packets = [0] * packets_num
            index = (num & 0xF0) >> 4

            packets[index] = packet.read()
            while 0 in packets:
                packet = Buffer(self.socket.recv(PACKET_SIZE))
                header = packet.read_int()
                if header == SINGLE:
                    raise PacketError('Wrong single packet')
                packet_id2 = packet.read_int()
                if packet_id2 != packet_id:
                    raise PacketError('Different packet id\'s')
                num = packet.read_byte()
                index = (num & 0xF0) >> 4
                packets[index] = packet.read()
            return Buffer(b''.join(packets))

    def ping(self):
        raise NotImplementedError('Ping is deprecated. Required new version of function')

    @staticmethod
    def __old_server_info(response):
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

    @staticmethod
    def __server_info(response):
        result = {
            'protocol': response.read_byte(),
            'name': response.read_string(),
            'map': response.read_string(),
            'folder': response.read_string(),
            'game': response.read_string(),
            'appid': response.read_short(),
            'players': response.read_byte(),
            'max_players': response.read_byte(),
            'bots': response.read_byte(),
            'server_type': chr(response.read_byte()),
            'enviroment': chr(response.read_byte()),
            'visibility': bool(response.read_byte()),
            'vac': bool(response.read_byte()),
            'version': response.read_string()
        }
        edf = response.read_byte()
        if edf & 0x80 == 1:
            result['port'] = response.read_short()
        if edf & 0x10 == 1:
            result['steamid'] = response.read_long_long()
        if edf & 0x40 == 1:
            result['spectator'] = {
                'port': response.read_short(),
                'name': response.read_string()
            }
        if edf & 0x20 == 1:
            result['keywords'] = response.read_string()
        if edf & 0x01 == 1:
            result['game_id'] = response.read_long_long()
        return result

    def a2s_info(self):
        self.socket.send(A2S_INFO_PACKET)
        response = self.read()
        header = response.read_byte()
        return self.__old_server_info(response) if header == ord('m') else self.__server_info(response)

    def get_challenge(self):
        self.socket.send(CHALLENGE_PACKET)
        response = self.read()
        header = response.read_byte()
        if header != ord('A'):
            raise PacketError('Wrong challenge packet\'s header')
        else:
            return response.read_int()

    def a2s_player(self, challenge):
        self.socket.send(build_packet_challenge(ord('U'), challenge))
        response = self.read()
        header = response.read_byte()
        if header != ord('D'):
            raise PacketError('Wrong header in a2s_player')
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

    def a2s_rules(self, challenge):
        self.socket.send(build_packet_challenge(ord('V'), challenge))
        response = self.read()
        if response.read_int() != -1:  # 4 FF bytes ain't required
            response.seek(response.tell() - 4)  # ... so we're gonna back to unreaded data

        header = response.read_byte()
        if header != ord('E'):
            raise PacketError('Wrong header in a2s_rules')
        size = response.read_short()
        rules = {}
        for i in range(size):
            key = response.read_string()
            value = response.read_string()
            rules[key] = value
        return {
            'size': size,
            'rules': rules
        }
