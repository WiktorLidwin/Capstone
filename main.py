import PySimpleGUI as sg  #gui
from sys import platform  #operating system info

"""
if platform == "linux" or platform == "linux2":
    #linux
    print("Linux user")
elif platform == "darwin":
    # OS X
    print("Mac user")
elif platform == "win32":
    # Windows...
    print("Windows user")
"""
image_home = './ButtonGraphics/home.png'
sg.theme('LightGreen2')  # Add a touch of color
# All the stuff inside your window.
layout = [  [sg.Text('LightSwitch', key='TITLE', text_color='green', font='Ubuntu 35', pad=((0,0),(50,15)))],
            [sg.Text('Username:', key='USERNAME', font='Ubuntu 10'), sg.InputText('', key='Username', size=(20,1))],
            [sg.Text('Password: ', key='PASSWORD', font='Ubuntu 10'), sg.InputText('', key='Password', password_char='*', size=(20,1))],
            [sg.Button('Login', key='_LOGIN_', button_color=('white', 'blue'), font='Ubuntu 10', size=(25,1), pad=((0,0),(10,5)))],
            [sg.Button('Create an account', key='_ACCOUNT_',button_color=('white', 'blue'), font='Ubuntu 10', size=(25,1), pad=((0,0),(5,10)))],
             ]

# Create the Window
main_window = sg.Window('LightSwitch::' + platform , layout,size=(600, 400), element_justification='c')
# Event Loop to process "events" and get the "values" of the inputs
devices_window_active = False
login = True #username and password acceptable
while True:
    event, values = main_window.read()
    if event in (sg.WIN_CLOSED, None): # if user closes window
        break
    if event == '_LOGIN_' and len(values['Username']) > 20 or len(values['Password']) > 20 or len(values['Username']) < 4 or len(values['Password']) < 4:
        sg.popup_error('Username and password must be less than 21 characters and at least 4 characters', title='Length Error')
        login = False
    else: 
        login = True
    if not devices_window_active and event == '_LOGIN_' and login:
        main_window.Hide()
        devices_window_active = True

        devices_layout = [
            [sg.Button('', key='_HOME_', button_color=(sg.theme_background_color(),sg.theme_background_color()),
                       image_filename=image_home, image_size=(50, 60), image_subsample=10, border_width=0), sg.Text('Devices', key='DEVICES', font='Ubuntu 35')],
            [sg.Button('Add a device', key='_ADD_', button_color=('white', 'blue'), pad=((5,10),(5,10)), font='Ubuntu 10', size=(25,1))],
        ]
        # New window after login
        devices_window = sg.Window('LightSwitch::' + platform + "::devices", devices_layout, size=(600, 400), element_justification='c')
        num_buttons = 0
        while True:
            event, values = devices_window.Read()
            if event in ('_HOME_', None):
                devices_window_active = False
                devices_window.Close()
                main_window.UnHide()
                break
            if event == '_ADD_':
                if num_buttons < 3:
                    num_buttons+=1
                devices_layout_1 = [
                    [sg.Button('', key='_HOME_', button_color=(sg.theme_background_color(),sg.theme_background_color()),
                       image_filename=image_home, image_size=(50, 60), image_subsample=10, border_width=0), sg.Text('Devices', key='DEVICES', font='Ubuntu 35')],
                    [sg.Button('Add a device', key='_ADD_', button_color=('white', 'blue'), font='Ubuntu 10', pad=((5,10),(5,10)), size=(25,1))],
                    [*[sg.InputOptionMenu(('Peripheral 0', 'Peripheral 1', 'Peripheral 2', 'Peripheral 3', 'Peripheral 4')) for i in range(num_buttons)]],
                    ]
                devices_window_with_device = sg.Window('LightSwitch::' + platform + "::devices", devices_layout_1, size=(600, 400), element_justification='c')
                devices_window.Close()
                devices_window = devices_window_with_device    
main_window.close()
