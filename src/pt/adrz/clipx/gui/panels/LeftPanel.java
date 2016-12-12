package pt.adrz.clipx.gui.panels;

import java.awt.BorderLayout;
import java.awt.FlowLayout;
import java.awt.event.ActionEvent;
import java.awt.event.ActionListener;
import java.awt.event.KeyEvent;
import java.awt.event.KeyListener;
import java.awt.event.MouseEvent;
import java.awt.event.MouseListener;

import javax.swing.JButton;
import javax.swing.JPanel;
import javax.swing.JScrollPane;
import javax.swing.JTextField;
import javax.swing.ListSelectionModel;
import javax.swing.ScrollPaneConstants;

import pt.adrz.clipx.ClipboardListener;
import pt.adrz.clipx.gui.list.LeftList;

public class LeftPanel extends ListPanel implements ActionListener, MouseListener, ClipboardListener, KeyListener {
	private static final long serialVersionUID = 1L;
	
	private LeftList list;
	private JScrollPane scrollPane;
	private JPanel southPanel;
	private JTextField inputNewElement;
	private JButton addButton;
	
	private Panels panels;

	public LeftPanel() {
		createList();
		createSouthElements();
		addComponents();
	}
	
	private void createList() {
		list = new LeftList();
		
		list.getModel().addElement("asdf");
		list.getModel().addElement("qwer");
		list.getModel().addElement("zxcv");

		list.setSelectedIndex(0);
		list.setVisibleRowCount(10);
		list.setSelectionMode(ListSelectionModel.SINGLE_SELECTION);

		list.addMouseListener(this);
		
		scrollPane = new JScrollPane();
		scrollPane = new JScrollPane(list,ScrollPaneConstants.VERTICAL_SCROLLBAR_ALWAYS,ScrollPaneConstants.HORIZONTAL_SCROLLBAR_NEVER);
	}
	
	private void createSouthElements() {
		inputNewElement = new JTextField(20);
		addButton = new JButton("ADD");

		southPanel = new JPanel();
		southPanel.setLayout(new FlowLayout());
		southPanel.add(inputNewElement);
		southPanel.add(addButton);
		
		addButton.addActionListener(this);
	}
	
	private void addComponents() {
		this.setLayout(new BorderLayout());
		this.add(list.getSearchField(),BorderLayout.NORTH);
		this.add(scrollPane,BorderLayout.CENTER);
		this.add(southPanel, BorderLayout.SOUTH);
	}
	
	public LeftList getList() {
		return list;
	}
	
	public void setPanels(Panels panels) {
		this.panels = panels;
	}
	
	public Panels getPanels() {
		return this.panels;
	}
	
	@Override
	public void mouseClicked(MouseEvent e) {
		if ( e.getClickCount() == 2 ) {
			
			int index = list.locationToIndex(e.getPoint());
			
			// double click in an empty space
			if ( index == -1 ) {
				list.clearSelection();
				return;
			}
			
			String selectedString = (String)list.getModel().getElementAt(index);
			
			list.getModel().switchVals(list.getModel().getItems().indexOf(selectedString), selectedString);
			list.setSelectedIndex(0);
			list.getSearchField().setText("");

			panels.changeClipBoard(selectedString);
			panels.getTextArea().setText(selectedString);
		}
	}

	@Override
	public void actionPerformed(ActionEvent e) {
		// TODO Auto-generated method stub
		
	}

	@Override
	public void activate() {
		Integer index = list.getSelectedIndex();
		if ( index == -1 ) return;
		String selectedString = (String)list.getModel().getElementAt(index);

		list.getModel().switchVals(list.getModel().getItems().indexOf(selectedString), selectedString);
		list.setSelectedIndex(0);
		list.getSearchField().setText("");

		panels.changeClipBoard(selectedString);
		panels.getTextArea().setText(selectedString);
	}

	@Override
	public void edit() {
		if ( panels.getTextArea().isEditable() )
			panels.getTextArea().setEditable(false);
		else
			panels.getTextArea().setEditable(true);
	}

	@Override
	public void delete() {
		try {
			Integer index = list.getSelectedIndex();
			list.getModel().remove(index);
			panels.getTextArea().setText("");
		}
		catch (IndexOutOfBoundsException eIndexOutBound) { 
			
		}
	}
	
	@Override
	public void showRightClickMenu(MouseEvent e) { 
		list.setSelectedIndex(list.locationToIndex(e.getPoint()));
		rightClickMenu.show(e.getComponent(), e.getX(), e.getY());
	}
	
	@Override
	public void newString(String copyString) {
		this.list.getModel().addElementTo(copyString, 0);
		panels.getTextArea().setText(copyString);
	}
	
	@Override
	public void keyPressed(KeyEvent e) {
		if (e.getKeyCode() == KeyEvent.VK_DELETE) {
			try {
				list.getModel().remove(list.getSelectedIndex());
				panels.getTextArea().setText("");
			}
			catch (IndexOutOfBoundsException eIndexOutBound) {
				
			}
		}
	}

	@Override
	public void keyTyped(KeyEvent e) { }

	@Override
	public void keyReleased(KeyEvent e) { }
}
