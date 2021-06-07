#!/usr/bin/env python3

from mididings import NOTEON
import mididings.engine as _engine
from mididings.event import NoteOnEvent, NoteOffEvent
import liblo as _liblo

# Carla callback opcodes - https://github.com/falkTX/Carla/blob/2a6a7de04f75daf242ae9d8c99b349ea7dc6ff7f/source/backend/CarlaBackend.h
ENGINE_CALLBACK_PLUGIN_REMOVED = 2
ENGINE_CALLBACK_PARAMETER_VALUE_CHANGED = 5

# Same as lpx-controller.py
# TODO get it from there directly
LPX_PORT_NAME_OUT = 'Launchpad X out'

# Color scales
COLORS_NORMAL =    [  0,  83, 127,  84,  61,  15,  14,  13]
COLORS_HIGHLIGHT = [103,   7,   6,   5,  60]

# based on https://github.com/dsacre/mididings/blob/master/mididings/extra/osc.py
class OSCInterface(object):
    """
    Allows controlling x42-stepseq LV2 plugin embedded in Carla
    when the Launchpad X is in Session mode.

    :param carla_port:
        the Carla OSC port to connect to.
    :param carla_plugin_id:
        index of the x42-stepseq plugin in the Carla rack
    :param list_port:
        a free port to receive events on.
    """
    def __init__(self, carla_port=22752, listen_port=22755): # TODO find free listen port
        self.carla_addr_tcp = _liblo.Address('127.0.0.1', carla_port, proto=_liblo.TCP)
        self.carla_addr_udp = _liblo.Address('127.0.0.1', carla_port, proto=_liblo.UDP)
        self.listen_port = listen_port
        self.carla_plugin_id = None
        self.current_step = 0
        self.grid_values = [0] * 8 * 8

    def on_start(self):
        print('starting osc')
        self.server_tcp = _liblo.ServerThread(self.listen_port, proto=_liblo.TCP)
        self.server_tcp.register_methods(self)
        self.server_tcp.start()
        self.server_udp = _liblo.ServerThread(self.listen_port, proto=_liblo.UDP)
        self.server_udp.register_methods(self)
        self.server_udp.start()
        _liblo.send(self.carla_addr_tcp, '/register', 'osc.tcp://127.0.0.1:%d/Carla' % self.listen_port)
        _liblo.send(self.carla_addr_udp, '/register', 'osc.udp://127.0.0.1:%d/Carla' % self.listen_port)
        # TODO query all current parameter values to set all buttons to the correct value


    def on_exit(self):
        # Registering with the full URL gives an error about the wrong owner, just the IP-address seems to work.
        #_liblo.send(self.carla_addr_tcp, '/unregister', 'osc.tcp://127.0.0.1:%d/Carla' % self.listen_port)
        #_liblo.send(self.carla_addr_udp, '/unregister', 'osc.udp://127.0.0.1:%d/Carla' % self.listen_port)
        _liblo.send(self.carla_addr_udp, '/unregister', '127.0.0.1')
        self.server_udp.stop()
        del self.server_udp
        _liblo.send(self.carla_addr_tcp, '/unregister', '127.0.0.1')
        self.server_tcp.stop()
        del self.server_tcp

    @_liblo.make_method('/Carla/info', 'iiiihiisssssss')
    def on_carla_info(self, path, args):
        if args[11] == 'http://gareus.org/oss/lv2/stepseq#s8n8':
            if self.carla_plugin_id is None:
                self.carla_plugin_id = args[0]
                print('Found Carla sequencer plugin at index %d.' % self.carla_plugin_id)
            elif self.carla_plugin_id != args[0]:
                print('New Carla Sequencer plugin ignored because we already found one.')

    @_liblo.make_method('/Carla/cb', 'iiiiifs')
    def on_carla_cb(self, path, args):
        # https://github.com/falkTX/Carla/blob/de8e0d3bd9cc4ab76cbea9f53352c92d89266ea2/source/frontend/carla_control.py#L337
        action, pluginId, value1, value2, value3, valuef, valueStr = args
        if action == ENGINE_CALLBACK_PLUGIN_REMOVED and pluginId == self.carla_plugin_id:
            self.on_carla_plugin_removed()
        elif action == ENGINE_CALLBACK_PARAMETER_VALUE_CHANGED and pluginId == self.carla_plugin_id:
            self.on_carla_value_changed(value1, valuef)

    def on_carla_plugin_removed(self):
        self.carla_plugin_id = None
        self.current_step = 0
        self.grid_values = [0] * 8 * 8
        print('Carla sequencer plugin removed.')

    def on_carla_value_changed(self, param, value):
        x, y = self._param_to_grid(param)
        self._set_lpx_grid(x, y, value)
        self.grid_values[y * 8 + x] = value

    @_liblo.make_method('/Carla/param', 'iif') # UDP
    def on_carla_param(self, path, args):
        # https://github.com/falkTX/Carla/blob/de8e0d3bd9cc4ab76cbea9f53352c92d89266ea2/source/frontend/carla_control.py#L534
        pluginId, paramId, paramValue = args
        if pluginId != self.carla_plugin_id: return
        print('param', path, args)
        if paramId == 7: # step position of sequencer plugin
            # update current step
            previous_step = self.current_step
            self.current_step = int(paramValue) - 1
            # update current and previous column on lpx
            for y in range(0, 8):
                self._set_lpx_grid(previous_step, y, self.grid_values[y * 8 + previous_step])
                self._set_lpx_grid(self.current_step, y, self.grid_values[y * 8 + self.current_step])

    def on_lpx_event(self, ev):
        if ev.channel == 1 and ev.type == NOTEON:
            note, value = ev.note, ev.velocity
            x, y = self._note_to_grid(note)
            if self.grid_values[y * 8 + x] > 0: value = 0 # clear if already set (same behaviour as step sequencer GUI)
            self._set_carla_grid(x, y, value)
            self.grid_values[y * 8 + x] = value
            return self._set_lpx_grid_event(x, y, value)

    def _set_carla_grid(self, x, y, value):
        if x is None or y is None: return
        if self.carla_plugin_id is None: return
        print('_set_carla_grid', x, y, value)
        param = self._grid_to_param(x, y)
        _liblo.send(self.carla_addr_tcp, '/Carla/%d/set_parameter_value' % self.carla_plugin_id, param, float(value))

    def _set_lpx_grid(self, x, y, value):
        print('_set_lpx_grid', x, y, value)
        ev = self._set_lpx_grid_event(x, y, value)
        if ev is not None:
            _engine.output_event(ev)

    def _set_lpx_grid_event(self, x, y, value):
        if x is None or y is None: return
        note = self._grid_to_note(x, y)
        color = self._value_to_color(value, x == self.current_step)
        if color > 0:
            return NoteOnEvent(LPX_PORT_NAME_OUT, 1, note, color)
        else:
            return NoteOffEvent(LPX_PORT_NAME_OUT, 1, note)

    def _param_to_grid(self, param):
        x = (param - 17) % 8
        y = int((param - 17) / 8)
        if x < 0 or y < 0 or x > 7 or y > 7:
            return None, None
        else:
            return x, y
        return x, y

    def _grid_to_param(self, x, y):
        return 17 + y * 8 + x

    def _note_to_grid(self, note):
        x = (note - 11) % 10
        y = 7 - int((note - 11) / 10) 
        if x < 0 or y < 0 or x > 7 or y > 7:
            return None, None
        else:
            return x, y

    def _grid_to_note(self, x, y):
        return 11 + (7 - y) * 10 + x

    def _value_to_color(self, value, highlight=False):
        scale = COLORS_HIGHLIGHT if highlight else COLORS_NORMAL
        if value < 1:
            return scale[0]
        else:
            return scale[1 + int(value * (len(scale) - 1) / 128)]

