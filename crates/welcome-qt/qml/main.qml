import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import org.rakuos.welcome 1.0

ApplicationWindow {
    id: root
    title: "Welcome to RakuOS"
    width: 720
    height: 580
    minimumWidth: 600
    minimumHeight: 500
    visible: true

    readonly property int totalPages: 5

    readonly property var setupPages: [
        {
            title:       "Gaming Setup",
            icon:        "🎮",
            description: "Install Steam and Lutris natively for the best gaming performance. " +
                         "Native packages give better Proton and Wine compatibility than Flatpak.",
            script:      "setup-gaming"
        },
        {
            title:       "Virtualization",
            icon:        "🖥",
            description: "Set up KVM/QEMU and virt-manager to run virtual machines. " +
                         "Ideal for running Windows or other Linux distros alongside RakuOS.",
            script:      "setup-virtualization"
        },
        {
            title:       "Local AI with Ollama",
            icon:        "🤖",
            description: "Install Ollama to run AI language models locally on your GPU. " +
                         "Keep your conversations private and use AI without the cloud.",
            script:      "setup-ollama"
        }
    ]

    // ── Backend ───────────────────────────────────────────────────────
    WelcomeBackend {
        id: backend
        onCloseRequested: Qt.quit()
        onLogRevisionChanged: readLog()
    }

    // Poll background-thread state every 300 ms via a lightweight Rust atomic read.
    Timer {
        interval: 300
        running:  true
        repeat:   true
        onTriggered: backend.pollScript()
    }

    // ── Log file reader (XMLHttpRequest — works for local file:// URLs) ──
    property string logFilePath: "file:///tmp/rakuos-welcome-qt.log"

    function readLog() {
        var xhr = new XMLHttpRequest()
        xhr.open("GET", logFilePath, true)
        xhr.onreadystatechange = function() {
            if (xhr.readyState === XMLHttpRequest.DONE) {
                logArea.text = xhr.responseText
                Qt.callLater(() => {
                    let fl = logScrollView.contentItem
                    if (fl) fl.contentY = Math.max(0, fl.contentHeight - logScrollView.height)
                })
            }
        }
        xhr.send()
    }

    // ── Root layout ───────────────────────────────────────────────────
    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // ── Page stack ────────────────────────────────────────────────
        StackLayout {
            id: pageStack
            Layout.fillWidth:  true
            Layout.fillHeight: true
            currentIndex: backend.currentPage

            // ── Page 0: Welcome ───────────────────────────────────────
            Item {
                ColumnLayout {
                    width: Math.min(parent.width - 120, 560)
                    anchors.horizontalCenter: parent.horizontalCenter
                    anchors.top: parent.top
                    anchors.topMargin: 40
                    spacing: 16

                    Image {
                        // Mirror the GTK frontend: white logo on dark themes, colour logo on light.
                        source: {
                            var bg = palette.window
                            var lum = 0.299 * bg.r + 0.587 * bg.g + 0.114 * bg.b
                            return lum < 0.5
                                ? "file:///usr/share/pixmaps/fedora_whitelogo_med.png"
                                : "file:///usr/share/pixmaps/fedora_logo_med.png"
                        }
                        Layout.preferredWidth: 200
                        Layout.preferredHeight: 80
                        Layout.alignment: Qt.AlignHCenter
                        fillMode: Image.PreserveAspectFit
                        smooth: true
                    }

                    Label {
                        text: "Welcome to RakuOS Linux"
                        font.pointSize: 20
                        font.bold: true
                        Layout.alignment: Qt.AlignHCenter
                    }

                    Label {
                        text: "The Hybrid Atomic Linux Desktop"
                        font.pointSize: 12
                        Layout.alignment: Qt.AlignHCenter
                        opacity: 0.7
                    }

                    Label {
                        text: "RakuOS combines the stability and security of an atomic " +
                              "immutable base with the flexibility of a traditional Linux " +
                              "distribution. Your system can never be broken by a bad update."
                        wrapMode: Text.WordWrap
                        Layout.fillWidth: true
                        horizontalAlignment: Text.AlignHCenter
                    }

                    Rectangle {
                        height: 1
                        Layout.fillWidth: true
                        color: palette.mid
                    }

                    Label {
                        text: "Find us online:"
                        Layout.alignment: Qt.AlignHCenter
                    }

                    RowLayout {
                        Layout.alignment: Qt.AlignHCenter
                        spacing: 12

                        Button {
                            text: "🌐 Website"
                            flat: true
                            onClicked: Qt.openUrlExternally("https://rakuos.org")
                        }
                        Button {
                            text: "💻 GitHub"
                            flat: true
                            onClicked: Qt.openUrlExternally("https://github.com/RakuOS")
                        }
                        Button {
                            text: "📦 SourceForge"
                            flat: true
                            onClicked: Qt.openUrlExternally("https://sourceforge.net/projects/rakuos/")
                        }
                    }
                }
            }

            // ── Pages 1–3: Setup pages ────────────────────────────────
            Repeater {
                model: root.setupPages

                Item {
                    required property var modelData
                    required property int index

                    readonly property bool isActivePage: backend.currentPage === (index + 1)

                    ColumnLayout {
                        anchors.centerIn: parent
                        spacing: 16
                        width: Math.min(parent.width - 120, 560)

                        Label {
                            text: modelData.icon
                            font.pointSize: 40
                            Layout.alignment: Qt.AlignHCenter
                        }

                        Label {
                            text: modelData.title
                            font.pointSize: 16
                            font.bold: true
                            Layout.alignment: Qt.AlignHCenter
                        }

                        Label {
                            text: modelData.description
                            wrapMode: Text.WordWrap
                            Layout.fillWidth: true
                            horizontalAlignment: Text.AlignHCenter
                        }

                        Button {
                            Layout.alignment: Qt.AlignHCenter
                            Layout.preferredWidth: 240
                            highlighted: true
                            enabled: !backend.scriptRunning && backend.scriptResult !== 1

                            text: {
                                if (!isActivePage)               return "Set Up " + modelData.title
                                if (backend.scriptResult === 1)  return "✓ Done"
                                if (backend.scriptResult === 2)  return "Retry"
                                if (backend.scriptRunning)       return "Installing…"
                                return "Set Up " + modelData.title
                            }

                            onClicked: backend.runScriptForPage(backend.currentPage)
                        }
                    }
                }
            }

            // ── Page 4: Done ──────────────────────────────────────────
            Item {
                ColumnLayout {
                    anchors.centerIn: parent
                    spacing: 16

                    Label {
                        text: "🎉"
                        font.pointSize: 48
                        Layout.alignment: Qt.AlignHCenter
                    }

                    Label {
                        text: "You're All Set!"
                        font.pointSize: 20
                        font.bold: true
                        Layout.alignment: Qt.AlignHCenter
                    }

                    Label {
                        text: "RakuOS is ready to use. You can always run additional setup\n" +
                              "at any time from the terminal using the rakuos command."
                        wrapMode: Text.WordWrap
                        Layout.preferredWidth: 480
                        horizontalAlignment: Text.AlignHCenter
                        Layout.alignment: Qt.AlignHCenter
                    }

                    Label {
                        text: "💡 Tip: Run  rakuos  in a terminal to see all available commands."
                        font.family: "monospace"
                        Layout.alignment: Qt.AlignHCenter
                    }
                }
            }
        } // StackLayout

        // ── Log area (visible while a script runs or after it finishes) ──
        Pane {
            // Only show on setup pages (1–3), never on Welcome (0) or Done (4).
            visible: (backend.scriptRunning || backend.scriptResult > 0)
                     && backend.currentPage > 0
                     && backend.currentPage < root.totalPages - 1
            Layout.fillWidth: true
            Layout.preferredHeight: 150
            padding: 4

            background: Rectangle {
                color: palette.base
                border.color: palette.mid
            }

            ScrollView {
                id: logScrollView
                anchors.fill: parent
                clip: true

                TextArea {
                    id: logArea
                    readOnly: true
                    font.family: "monospace"
                    font.pointSize: 9
                    wrapMode: TextArea.WordWrap
                    background: null
                    padding: 6
                }
            }
        }

        // ── Separator ─────────────────────────────────────────────────
        Rectangle {
            height: 1
            Layout.fillWidth: true
            color: palette.mid
        }

        // ── Navigation bar ────────────────────────────────────────────
        RowLayout {
            Layout.fillWidth: true
            Layout.leftMargin:   24
            Layout.rightMargin:  24
            Layout.topMargin:    12
            Layout.bottomMargin: 12
            spacing: 8

            RowLayout {
                spacing: 6
                Repeater {
                    model: root.totalPages
                    Label {
                        text: "●"
                        font.pointSize: 8
                        color: index === backend.currentPage ? palette.highlight : palette.mid
                    }
                }
            }

            Item { Layout.fillWidth: true }

            Button {
                text: "← Back"
                visible: backend.currentPage > 0
                enabled: !backend.scriptRunning
                onClicked: backend.backPage()
            }

            Button {
                text: backend.currentPage === root.totalPages - 1 ? "Finish" : "Next →"
                highlighted: true
                enabled: !backend.scriptRunning
                onClicked: {
                    if (backend.currentPage === root.totalPages - 1)
                        backend.finishSetup()
                    else
                        backend.nextPage()
                }
            }
        }
    }
}
